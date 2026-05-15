//! `RequireCapability` axum extractor — the load-bearing piece of Phase 5.
//!
//! Every admin route declares the capability it needs by adding
//! `RequireCapability(Capability::X)` to its handler signature. The extractor:
//!
//! 1. Pulls the authenticated user via `api_core::RequestPrincipal` — Phase 2
//!    contract: principal kind is re-derived from the trusted `users` table on
//!    every request; JWT `roles` / `kind` claims are NEVER trusted server-side.
//! 2. Enforces `principal_kind == Platform` via the strongly-typed
//!    [`PrincipalKind::Platform`].
//! 3. Looks up an active capability grant.
//! 4. Verifies recent MFA (15-minute window).
//! 5. Writes an audit row — successful or denied.
//! 6. Inserts the capability into request extensions so downstream code
//!    can re-check without another round-trip.
//!
//! Failure modes:
//!   * Step 1 fails → 401 `unauthenticated`.
//!   * Step 2 fails → 403 `not_platform_principal`. (Audited as denied.)
//!   * Step 3 fails → 403 `missing_capability`. (Audited as denied.)
//!   * Step 4 fails → 401 `{ "error": "mfa_required" }`. (Audited as denied.)
//!
//! All denials are audited so operators can spot abuse via stolen credentials.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::sync::Arc;

use api_core::extractors::principal::RequestPrincipal;
use api_core::extractors::tenant::TenantMembershipProvider;

use crate::{
    AdminError, AuditOutcome, AuditWriter, Capability, CapabilityGrantsRepository, MfaRecency,
};

/// Bundle of dependencies the extractor needs. Stored in axum's request
/// extensions by the route layer (e.g. `AppState::admin_deps`).
#[derive(Clone)]
pub struct AdminDeps {
    pub grants: Arc<dyn CapabilityGrantsRepository>,
    pub mfa: Arc<dyn MfaRecency>,
    pub audit: Arc<dyn AuditWriter>,
}

impl AdminDeps {
    pub fn new(
        grants: Arc<dyn CapabilityGrantsRepository>,
        mfa: Arc<dyn MfaRecency>,
        audit: Arc<dyn AuditWriter>,
    ) -> Self {
        Self { grants, mfa, audit }
    }
}

/// `axum` extractor enforcing a single capability. Construct with the
/// capability you require, e.g. `RequireCapability(Capability::AuditRead)`.
///
/// The extractor returns itself so handlers can read `.0` to know which
/// capability was just satisfied (handy when a handler is reused under
/// multiple capabilities).
#[derive(Debug, Clone, Copy)]
pub struct RequireCapability(pub Capability);

impl<S> FromRequestParts<S> for RequireCapability
where
    S: TenantMembershipProvider,
{
    type Rejection = AdminError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Pull the capability target out of the extension that the route
        // builder set. Routes that use this extractor MUST also call
        // `.layer(Extension(RequiredCapabilityMarker(cap)))` — but we make
        // this ergonomic by pre-loading via a tower layer in practice. For
        // direct call sites we read it back from extensions.
        let required = parts
            .extensions
            .get::<RequiredCapabilityMarker>()
            .map(|m| m.0)
            .ok_or_else(|| {
                AdminError::Internal(
                    "RequireCapability: marker missing — wrap route with require_capability layer"
                        .into(),
                )
            })?;

        let deps = parts
            .extensions
            .get::<AdminDeps>()
            .cloned()
            .ok_or_else(|| AdminError::Internal("AdminDeps missing from extensions".into()))?;

        // Capture request metadata for the audit row before we move on.
        let ip = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        let user_agent = parts
            .headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Step 1: authenticated principal — Phase 2 contract.
        //
        // `RequestPrincipal` re-derives `principal_kind` from the trusted
        // `users` table on every request. JWT `roles` / `kind` claims are
        // accepted only as informational hints; they are NEVER trusted as
        // authority server-side (defends leaks #10/#11).
        let principal = RequestPrincipal::from_request_parts(parts, state)
            .await
            .map_err(|(_status, _msg)| AdminError::Unauthenticated)?;

        // Step 2: platform principal.
        //
        // Strongly-typed: `PrincipalKind::Platform` from the `users.principal_kind`
        // column. Replaces the previous `AuthUser::is_platform_admin()` check
        // which trusted the JWT `roles` claim — a token-claim-forgery risk.
        if !principal.is_platform() {
            audit_denied(
                &deps,
                Some(principal.user_id),
                required,
                ip.as_deref(),
                user_agent.as_deref(),
            )
            .await;
            return Err(AdminError::NotPlatformPrincipal);
        }

        // Step 3: capability grant.
        let has = deps.grants.user_has(principal.user_id, required).await?;
        if !has {
            audit_denied(
                &deps,
                Some(principal.user_id),
                required,
                ip.as_deref(),
                user_agent.as_deref(),
            )
            .await;
            return Err(AdminError::MissingCapability(required));
        }

        // Step 4: recent MFA. We re-check `mfa_required_for(user, cap)` so a
        // narrowly-scoped grant (e.g. an automation user) can opt out. The
        // default is TRUE; never opt out without an explicit, audited reason.
        let mfa_required = deps
            .grants
            .mfa_required_for(principal.user_id, required)
            .await?;
        if mfa_required {
            let fresh = deps.mfa.recent_mfa(principal.user_id).await?;
            if !fresh {
                audit_denied(
                    &deps,
                    Some(principal.user_id),
                    required,
                    ip.as_deref(),
                    user_agent.as_deref(),
                )
                .await;
                return Err(AdminError::MfaRequired(crate::MfaRequired::default()));
            }
        }

        // Step 5: audit success.
        let _ = deps
            .audit
            .record(
                Some(principal.user_id),
                required,
                AuditOutcome::Allowed,
                None,
                None,
                None,
                ip.as_deref(),
                user_agent.as_deref(),
            )
            .await;

        // Step 6: inject the satisfied capability into extensions.
        parts.extensions.insert(SatisfiedCapability(required));

        Ok(RequireCapability(required))
    }
}

/// Helper to record a denied audit row best-effort. We swallow audit errors
/// here because the extractor's primary job is to refuse the request — we
/// must not leak audit-store outages as 500s on a 401/403 path.
async fn audit_denied(
    deps: &AdminDeps,
    actor_id: Option<uuid::Uuid>,
    capability: Capability,
    ip: Option<&str>,
    user_agent: Option<&str>,
) {
    if let Err(e) = deps
        .audit
        .record(
            actor_id,
            capability,
            AuditOutcome::Denied,
            None,
            None,
            None,
            ip,
            user_agent,
        )
        .await
    {
        tracing::error!(
            target: "admin_core",
            error = %e,
            capability = capability.as_str(),
            "failed to record denied-audit event"
        );
    }
}

/// Marker placed in extensions by the layer to tell the extractor what
/// capability is required. Only consumed by the extractor; not user-facing.
#[derive(Debug, Clone, Copy)]
pub struct RequiredCapabilityMarker(pub Capability);

/// Inserted into extensions on success — handlers can read it to know they
/// were authorized for `cap`.
#[derive(Debug, Clone, Copy)]
pub struct SatisfiedCapability(pub Capability);

/// Tower layer factory: wraps a router branch so it requires `cap`.
///
/// Usage:
/// ```ignore
/// let admin_routes = Router::new()
///     .route("/agencies", get(list_agencies))
///     .layer(require_capability(Capability::AgenciesRead));
/// ```
pub fn require_capability(cap: Capability) -> axum::Extension<RequiredCapabilityMarker> {
    axum::Extension(RequiredCapabilityMarker(cap))
}

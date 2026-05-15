//! AuthPolicyEnforcer — Defense N2 / Phase 2 leak #13.
//!
//! Re-evaluates the per-organization [`AuthPolicy`] at every privilege change
//! BEFORE the action proceeds. The enforcer is the single application-layer
//! seam every privileged write goes through, so the org's policy is always
//! resolved at the moment of the action — never against a stale per-session
//! cache or against the wrong tenant's defaults.
//!
//! # Wired surfaces
//!
//! * `routes::admin::memberships::invite` — enforces `require_email_verification`
//!  before issuing an invite to an unverified user, and re-validates the org's
//!  policy is loadable before any audit row is written.
//! * `routes::admin::memberships::revoke` — soft check (the policy is read; an
//!  unloadable policy is treated as a hard fail).
//! * `routes::admin::capabilities::grant` — enforces `require_email_verification`
//!  on the grantee before any capability row is written.
//! * `routes::admin::capabilities::revoke` — platform-scoped liveness check
//!    (D2.1 follow-up): see [`check_capability_revoke`] /
//!    [`check_platform_action`]. Capability rows have no `organization_id` —
//!  they are platform-scoped per
//!  `docs/multitenancy/decisions/capability-platform-scope.md` — so the
//!  enforcer reads the platform-default policy and asserts platform-action
//!  invariants (the principal-kind + recent-MFA gates are owned by the
//!  `RequireCapability` extractor; this method is the policy-loadability
//!  liveness check).
//! * `routes::admin::users::set_principal_kind` — re-checks the target's
//!  org-effective policy is loadable before promotion.
//! * `routes::auth::login` — `mfa_required_for_roles` enforcement against the
//!  union of the user's active org memberships (D2.2). Falls back to the
//!  platform-default policy for users with no memberships.
//! * `routes::auth::reset_password` — `validate_password` against the user's
//!  org-of-record policy (or platform default for org-less users) (D2.2).
//!  Reality-server's password change is owned by N1's identity stack.

use db::models::AuthPolicy;
use db::repositories::{MembershipRepository, UserRepository};
use db::DbPool;
use thiserror::Error;
use uuid::Uuid;

/// Reasons an enforcement check can fail. Mapped to HTTP status by the caller.
#[derive(Debug, Error)]
pub enum AuthPolicyError {
    #[error("auth policy lookup failed: {0}")]
    Lookup(#[from] sqlx::Error),
    /// The grantee user does not exist (or is deleted). Caller should map
    /// to 404 / 422 depending on the surface.
    #[error("target user {0} not found")]
    UserNotFound(Uuid),
    /// The org's policy requires email verification before a privilege grant
    /// and the target user has not verified their email.
    #[error("org policy requires verified email before this action")]
    EmailNotVerified,
    /// The supplied password does not satisfy the org's password policy.
    #[error("password policy violations: {0:?}")]
    PasswordPolicy(Vec<String>),
    /// MFA is required for this role and the caller did not present a valid
    /// MFA challenge. Reserved for the login follow-up surface.
    #[error("MFA required for role '{0}'")]
    MfaRequired(String),
}

/// Centralized policy gate. Holds a pool reference and a pair of repository
/// shims for the lookups it performs. Cheap to clone (all fields are pool
/// handles).
#[derive(Clone)]
pub struct AuthPolicyEnforcer {
    pool: DbPool,
}

impl AuthPolicyEnforcer {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Resolve the effective auth policy for an organization. Thin wrapper
    /// around [`AuthPolicy::for_org`] to keep call sites pool-agnostic.
    pub async fn policy_for(&self, org_id: Uuid) -> Result<AuthPolicy, AuthPolicyError> {
        AuthPolicy::for_org(&self.pool, org_id)
            .await
            .map_err(AuthPolicyError::from)
    }

    /// Resolve the **platform-default** auth policy. Used by surfaces that
    /// operate without a tenant in scope (capability grant/revoke, login by a
    /// user with no active memberships, etc.). The default mirrors Phase 0
    /// platform behavior — adopting per-org policies is purely additive.
    pub fn platform_default_policy(&self) -> AuthPolicy {
        AuthPolicy::default()
    }

    /// Resolve the **strictest** policy across a set of orgs. The semantic
    /// chosen here is "any org demands it → it is demanded": a user who is a
    /// member of two orgs where ONE requires MFA for their role must MFA at
    /// login. This mirrors how a user can never escape the most restrictive
    /// tenant they belong to. Returns the platform default if `org_ids` is
    /// empty.
    async fn strictest_policy_across(
        &self,
        org_ids: &[Uuid],
    ) -> Result<AuthPolicy, AuthPolicyError> {
        if org_ids.is_empty() {
            return Ok(self.platform_default_policy());
        }

        let mut effective = self.platform_default_policy();
        for org in org_ids {
            let p = self.policy_for(*org).await?;
            // Take the max of every numeric / boolean / set field so the
            // result is the most restrictive combination. This is the
            // "tighten, never loosen" rule.
            effective.min_password_length =
                effective.min_password_length.max(p.min_password_length);
            effective.require_uppercase = effective.require_uppercase || p.require_uppercase;
            effective.require_digit = effective.require_digit || p.require_digit;
            effective.require_symbol = effective.require_symbol || p.require_symbol;
            effective.require_email_verification =
                effective.require_email_verification || p.require_email_verification;
            // Union the MFA-required role list.
            for r in p.mfa_required_for_roles {
                if !effective
                    .mfa_required_for_roles
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(&r))
                {
                    effective.mfa_required_for_roles.push(r);
                }
            }
            // For session-cap, treat 0 as "no cap". Otherwise take the min
            // (tightest) non-zero value.
            effective.max_session_age_minutes =
                match (effective.max_session_age_minutes, p.max_session_age_minutes) {
                    (0, x) => x,
                    (x, 0) => x,
                    (a, b) => a.min(b),
                };
        }
        Ok(effective)
    }

    /// Check a password change / set against the user's effective org policy.
    /// Caller passes the plaintext (only) — we do NOT hash here; that is the
    /// caller's job. Resolves the user's memberships and applies the
    /// strictest policy across them; falls back to the platform default for
    /// org-less users.
    pub async fn check_password_change(
        &self,
        user_id: Uuid,
        plaintext: &str,
    ) -> Result<(), AuthPolicyError> {
        let mem_repo = MembershipRepository::new(self.pool.clone());
        let memberships = mem_repo.list_for_user(user_id).await?;
        let org_ids: Vec<Uuid> = memberships.iter().map(|m| m.organization_id).collect();
        let policy = self.strictest_policy_across(&org_ids).await?;
        policy
            .validate_password(plaintext)
            .map_err(AuthPolicyError::PasswordPolicy)
    }

    /// Re-evaluate before a membership grant proceeds. The org's effective
    /// policy is loaded fresh on every call (no caching) so a policy edit in
    /// the same transaction window is honored on the next grant.
    ///
    /// Specifically enforces:
    ///   * `require_email_verification` — the target must have a non-null
    ///     `email_verified_at`. This stops a tenant from silently inheriting
    ///     a member who side-stepped the verification flow.
    pub async fn check_membership_grant(
        &self,
        org_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), AuthPolicyError> {
        let policy = self.policy_for(org_id).await?;

        if policy.require_email_verification {
            let user_repo = UserRepository::new(self.pool.clone());
            let user = user_repo
                .find_by_id(target_user_id)
                .await?
                .ok_or(AuthPolicyError::UserNotFound(target_user_id))?;
            if user.email_verified_at.is_none() {
                return Err(AuthPolicyError::EmailNotVerified);
            }
        }

        Ok(())
    }

    /// Re-evaluate before a membership revoke proceeds. The current policy
    /// has no revoke-specific clauses, but loading it acts as a liveness
    /// check (a corrupted policy row aborts the revoke instead of silently
    /// proceeding under stale rules).
    pub async fn check_membership_revoke(
        &self,
        org_id: Uuid,
        _target_user_id: Uuid,
    ) -> Result<(), AuthPolicyError> {
        let _policy = self.policy_for(org_id).await?;
        Ok(())
    }

    /// Re-evaluate before a capability grant proceeds. Same gate as
    /// `check_membership_grant` — the grantee must have a verified email if
    /// the org demands one. The capability gate itself (Phase 5) handles the
    /// no-self-grant + escalation rules.
    pub async fn check_capability_grant(
        &self,
        org_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), AuthPolicyError> {
        let policy = self.policy_for(org_id).await?;
        if policy.require_email_verification {
            let user_repo = UserRepository::new(self.pool.clone());
            let user = user_repo
                .find_by_id(target_user_id)
                .await?
                .ok_or(AuthPolicyError::UserNotFound(target_user_id))?;
            if user.email_verified_at.is_none() {
                return Err(AuthPolicyError::EmailNotVerified);
            }
        }
        Ok(())
    }

    /// Capability grants are platform-scoped (no org_id on the grant row),
    /// but the grantee belongs to one or more orgs whose policies we still
    /// want to honor. This convenience picks the grantee's first active
    /// membership and runs `check_capability_grant` against it. Returns
    /// `Ok(())` immediately if the grantee has no memberships (platform-only
    /// principal — the grant is governed by the platform default).
    pub async fn check_capability_grant_for_user(
        &self,
        target_user_id: Uuid,
    ) -> Result<(), AuthPolicyError> {
        let mem_repo = MembershipRepository::new(self.pool.clone());
        let memberships = mem_repo.list_for_user(target_user_id).await?;
        if let Some(m) = memberships.first() {
            self.check_capability_grant(m.organization_id, target_user_id)
                .await
        } else {
            Ok(())
        }
    }

    /// Re-evaluate before a `principal_kind` transition. The target's
    /// EFFECTIVE org policy (best-effort: any active membership; falls back
    /// to default if none) gates the transition. We do NOT block on
    /// `require_email_verification` here — the platform-promotion path is
    /// already capability-gated by `PrincipalKindEscalate` and audit-checked
    /// by N3 — but we DO load the policy as a liveness check.
    pub async fn check_principal_kind_change(
        &self,
        target_user_id: Uuid,
    ) -> Result<(), AuthPolicyError> {
        // Best-effort org resolution: pick any active membership for a
        // policy-load liveness check. Platform principals with no
        // memberships skip the check entirely (their policy is platform
        // defaults by definition).
        let mem_repo = MembershipRepository::new(self.pool.clone());
        let memberships = mem_repo.list_for_user(target_user_id).await?;
        if let Some(m) = memberships.first() {
            let _policy = self.policy_for(m.organization_id).await?;
        }
        Ok(())
    }

    /// Login-time enforcement (D2.2). Loads the **union** of per-org policies
    /// across the user's active memberships and asserts:
    ///
    ///   * If ANY org policy requires MFA for ANY of the user's role(s) in
    ///     that org and `mfa_presented == false`, returns
    ///     [`AuthPolicyError::MfaRequired`]. The role in the error string is
    ///     the first matching role found, useful for telemetry and the 401
    ///     `mfa_required` response body.
    ///
    /// Email-verification at login is already enforced by the legacy
    /// `User::is_verified` check before this method is called — we do NOT
    /// re-emit `EmailNotVerified` here to avoid changing the login error
    /// shape twice (the existing handler returns `EMAIL_NOT_VERIFIED`).
    ///
    /// For users with no active memberships (platform-only / fresh signups),
    /// the platform default policy applies. The default does not require MFA
    /// for any role, so the call short-circuits to `Ok(())`.
    pub async fn check_login(
        &self,
        user_id: Uuid,
        mfa_presented: bool,
    ) -> Result<(), AuthPolicyError> {
        let mem_repo = MembershipRepository::new(self.pool.clone());
        let memberships = mem_repo.list_for_user(user_id).await?;

        // Walk every (org, role) pair and stop on the first MFA demand the
        // user has NOT satisfied. Platform-default applies when the user has
        // no memberships; that default carries no MFA roles, so the loop is
        // a no-op there.
        for m in &memberships {
            let policy = self.policy_for(m.organization_id).await?;
            if policy.mfa_required_for(&m.role) && !mfa_presented {
                return Err(AuthPolicyError::MfaRequired(m.role.clone()));
            }
        }
        Ok(())
    }

    /// Re-evaluate before a capability **revoke** proceeds. Capability rows
    /// are platform-scoped (no `organization_id` column); see
    /// `docs/multitenancy/decisions/capability-platform-scope.md`. The
    /// principal-kind + recent-MFA invariants are owned by the
    /// `RequireCapability` extractor, which has already run by the time this
    /// method is called. This call therefore loads the **platform-default**
    /// policy as a liveness check — symmetric to `check_membership_revoke`'s
    /// org-policy load — so a corrupted policy default-resolution path
    /// aborts the revoke instead of silently proceeding.
    pub async fn check_capability_revoke(
        &self,
        actor_user_id: Uuid,
    ) -> Result<(), AuthPolicyError> {
        self.check_platform_action(actor_user_id).await
    }

    /// Generic platform-scoped action gate. Loads the platform-default policy
    /// as a liveness check (the actual platform invariants — `Platform`
    /// principal-kind + recent MFA — are enforced by the `RequireCapability`
    /// extractor before any handler that uses this method is reached).
    ///
    /// This method exists so platform-scoped surfaces (capability revoke,
    /// future tenant-lifecycle actions) have a single, named seam in the
    /// enforcer they can call without inventing an `org_id` for a row that
    /// has none.
    pub async fn check_platform_action(&self, _actor_user_id: Uuid) -> Result<(), AuthPolicyError> {
        // Load the platform default as a liveness check. The `Default`
        // implementation is infallible, so this is mostly a documentation
        // anchor today — but if `platform_default_policy` ever grows a DB
        // backing (e.g. a `platform_auth_policy` table), this seam will
        // surface that lookup's errors as `AuthPolicyError::Lookup` without
        // any handler-side change.
        let _policy = self.platform_default_policy();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_verification_error_carries_meaning() {
        let e = AuthPolicyError::EmailNotVerified;
        assert!(format!("{e}").contains("verified email"));
    }

    #[test]
    fn password_policy_error_lists_violations() {
        let e = AuthPolicyError::PasswordPolicy(vec![
            "must contain a digit".into(),
            "must be at least 12 characters".into(),
        ]);
        let s = format!("{e}");
        assert!(s.contains("digit"));
        assert!(s.contains("at least 12"));
    }
}

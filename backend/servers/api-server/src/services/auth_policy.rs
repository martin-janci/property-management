//! AuthPolicyEnforcer — Defense N2 / Phase 2 leak #13.
//!
//! Re-evaluates the per-organization [`AuthPolicy`] at every privilege change
//! BEFORE the action proceeds. The enforcer is the single application-layer
//! seam every privileged write goes through, so the org's policy is always
//! resolved at the moment of the action — never against a stale per-session
//! cache or against the wrong tenant's defaults.
//!
//! # Wired surfaces (this PR)
//!
//! * `routes::admin::memberships::invite` — enforces `require_email_verification`
//!    before issuing an invite to an unverified user, and re-validates the org's
//!    policy is loadable before any audit row is written.
//! * `routes::admin::memberships::revoke` — soft check (the policy is read; an
//!    unloadable policy is treated as a hard fail).
//! * `routes::admin::capabilities::grant` — enforces `require_email_verification`
//!    on the grantee before any capability row is written.
//! * `routes::admin::users::set_principal_kind` — re-checks the target's
//!    org-effective policy is loadable before promotion.
//!
//! # TODO surfaces (follow-up)
//!
//! * Login (api-server `/auth/login`) — `mfa_required_for_roles` enforcement.
//! * Password change / reset — `validate_password` against the user's effective
//!    org policy. Reality-server side belongs to N1's identity stack.
//! * Capability revoke — symmetric counterpart to grant; left as a follow-up.
//!
//! See call sites for `// TODO(N2-followup):` markers.

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

    /// Check a password change / set against `org_id`'s policy. Caller passes
    /// the plaintext (only) — we do NOT hash here; that is the caller's job.
    /// Used by password-reset / change handlers (TODO follow-up).
    pub async fn check_password_change(
        &self,
        org_id: Uuid,
        plaintext: &str,
    ) -> Result<(), AuthPolicyError> {
        let policy = self.policy_for(org_id).await?;
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

    /// Reserved for the login surface. MFA enforcement at login was paused
    /// pending the login refactor; surfaces should call this once the flow
    /// is wired through this enforcer.
    pub async fn check_login(
        &self,
        _org_id: Uuid,
        _user_id: Uuid,
        _role: &str,
        _mfa_presented: bool,
    ) -> Result<(), AuthPolicyError> {
        // TODO(N2-followup): wire MFA-at-login here. Pseudocode:
        //
        //   let policy = self.policy_for(org_id).await?;
        //   if policy.mfa_required_for(role) && !mfa_presented {
        //       return Err(AuthPolicyError::MfaRequired(role.to_string()));
        //   }
        //   Ok(())
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

//! Per-organization authentication policy (Defense N2 / Phase 2 leak #13).
//!
//! Each organization can override the platform's default authentication policy
//! (password requirements, MFA, email-verification rules, session lifetime).
//! Stored as a single JSONB row in `org_auth_policies` keyed by
//! `organization_id`. Absent rows mean "use the platform defaults" — the
//! [`AuthPolicy::for_org`] loader handles the fall-back.
//!
//! The `AuthPolicyEnforcer` service in api-server re-evaluates the relevant
//! policy at every privilege change (membership grant/revoke, capability
//! grant/revoke, principal_kind transition, password change, login). This
//! defends leak #13 — "policy resolved under wrong tenant" — by ensuring
//! the per-org rules apply at the moment of the privileged action, not at
//! some earlier session-anchored check.

use crate::DbPool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Error as SqlxError;
use utoipa::ToSchema;
use uuid::Uuid;

/// Effective per-org authentication policy.
///
/// All numeric fields use sensible platform defaults via `AuthPolicy::default`.
/// Per-org overrides are deserialized from the `policy` JSONB column with
/// `serde(default)` so partial overrides are valid (an org may set ONLY
/// `mfa_required_for_roles` and inherit everything else).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct AuthPolicy {
    /// Minimum password length in characters.
    pub min_password_length: u32,
    /// Require at least one uppercase letter in passwords.
    pub require_uppercase: bool,
    /// Require at least one digit in passwords.
    pub require_digit: bool,
    /// Require at least one symbol (non-alphanumeric) in passwords.
    pub require_symbol: bool,
    /// New members must have a verified email before a membership can be granted.
    pub require_email_verification: bool,
    /// Roles for which MFA is mandatory at login. Compared lowercase against
    /// the role string in `user_memberships`.
    pub mfa_required_for_roles: Vec<String>,
    /// Maximum session age in minutes. Sessions exceeding this must be
    /// re-authenticated. `0` means "no enforced cap" (rely on JWT exp only).
    pub max_session_age_minutes: u32,
}

impl Default for AuthPolicy {
    /// The platform-wide defaults applied when no per-org row exists. These
    /// MIRROR the historical platform behavior (Phase 0 / pre-multitenant),
    /// so adopting per-org policies is purely additive — the default is the
    /// status quo.
    fn default() -> Self {
        Self {
            // 8-char minimum aligns with the existing `AuthService::register`
            // validation (Phase 0); we centralize it here.
            min_password_length: 8,
            require_uppercase: false,
            require_digit: false,
            require_symbol: false,
            // Email verification was historically optional; keep it that way
            // for the platform default. Orgs that want stricter membership
            // gating opt in by setting `require_email_verification = true`.
            require_email_verification: false,
            mfa_required_for_roles: Vec::new(),
            // 0 = unlimited (rely on JWT exp). Orgs override to clamp.
            max_session_age_minutes: 0,
        }
    }
}

impl AuthPolicy {
    /// Load the effective policy for `org_id`. Falls back to the default if
    /// no `org_auth_policies` row exists.
    ///
    /// Issued under a SUPER-ADMIN RLS context — the policy is read as an
    /// authoritative platform rule, not a tenant-scoped resource. Callers in
    /// the application layer should ALWAYS read via this function instead of
    /// `SELECT ... FROM org_auth_policies` directly so the fall-back contract
    /// is honored.
    pub async fn for_org(pool: &DbPool, org_id: Uuid) -> Result<Self, SqlxError> {
        let mut conn = pool.acquire().await?;

        // Authority lookup — read as super-admin so we are not blocked by
        // tenant-isolation RLS when the caller is on a different tenant
        // context (or no tenant context at all, e.g. login).
        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let raw: Option<Value> = sqlx::query_scalar(
            r#"SELECT policy FROM org_auth_policies WHERE organization_id = $1"#,
        )
        .bind(org_id)
        .fetch_optional(&mut *conn)
        .await?;

        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::warn!(
                error = %e,
                org_id = %org_id,
                "Failed to clear RLS context after AuthPolicy::for_org"
            );
        }

        // Empty / absent / unparseable rows fall back to the default.
        // Partial overrides parse cleanly thanks to `#[serde(default)]`.
        let policy = match raw {
            None => Self::default(),
            Some(v) if v.is_null() => Self::default(),
            Some(Value::Object(ref m)) if m.is_empty() => Self::default(),
            Some(v) => serde_json::from_value::<AuthPolicy>(v).unwrap_or_else(|err| {
                tracing::warn!(
                    error = %err,
                    org_id = %org_id,
                    "Malformed org_auth_policies.policy; falling back to default"
                );
                Self::default()
            }),
        };

        Ok(policy)
    }

    /// Validate a password against this policy. Returns `Ok(())` on success
    /// or a sequence of human-readable failure reasons (one per rule violated).
    pub fn validate_password(&self, plaintext: &str) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();
        if (plaintext.chars().count() as u32) < self.min_password_length {
            errs.push(format!(
                "password must be at least {} characters",
                self.min_password_length
            ));
        }
        if self.require_uppercase && !plaintext.chars().any(|c| c.is_ascii_uppercase()) {
            errs.push("password must contain an uppercase letter".into());
        }
        if self.require_digit && !plaintext.chars().any(|c| c.is_ascii_digit()) {
            errs.push("password must contain a digit".into());
        }
        if self.require_symbol && !plaintext.chars().any(|c| !c.is_alphanumeric()) {
            errs.push("password must contain a symbol".into());
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    /// Whether MFA is required at login for `role` (case-insensitive match).
    pub fn mfa_required_for(&self, role: &str) -> bool {
        let role_lc = role.to_ascii_lowercase();
        self.mfa_required_for_roles
            .iter()
            .any(|r| r.eq_ignore_ascii_case(&role_lc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_password_validation_only_checks_length() {
        let p = AuthPolicy::default();
        assert!(p.validate_password("short").is_err());
        assert!(p.validate_password("12345678").is_ok());
        assert!(p.validate_password("longenough").is_ok());
    }

    #[test]
    fn strict_policy_enforces_all_classes() {
        let p = AuthPolicy {
            min_password_length: 10,
            require_uppercase: true,
            require_digit: true,
            require_symbol: true,
            ..Default::default()
        };
        assert!(p.validate_password("Password1!").is_ok());
        let errs = p.validate_password("password").unwrap_err();
        assert!(errs.iter().any(|e| e.contains("uppercase")));
        assert!(errs.iter().any(|e| e.contains("digit")));
        assert!(errs.iter().any(|e| e.contains("symbol")));
    }

    #[test]
    fn mfa_required_for_is_case_insensitive() {
        let p = AuthPolicy {
            mfa_required_for_roles: vec!["Manager".into()],
            ..Default::default()
        };
        assert!(p.mfa_required_for("manager"));
        assert!(p.mfa_required_for("MANAGER"));
        assert!(!p.mfa_required_for("owner"));
    }

    #[test]
    fn partial_override_uses_serde_default() {
        // Missing keys must fall back to the default — this is what makes
        // the JSONB column robust against drift.
        let json = serde_json::json!({ "min_password_length": 16 });
        let p: AuthPolicy = serde_json::from_value(json).unwrap();
        assert_eq!(p.min_password_length, 16);
        assert_eq!(p.require_uppercase, AuthPolicy::default().require_uppercase);
    }
}

//! CROSS-CUTTING authorization helpers (EPIC-ACC-16.2 / 01.3). OWNED BY: BE-Platform.
//!
//! EVERY route file calls these to enforce RBAC server-side (UC-ACC-16.2:
//! denied actions return 403 regardless of UI state). Keep the signatures
//! STABLE — other coders depend on them. accounting-core stays transport-
//! agnostic, so these take `common::TenantRole` (the value behind
//! `AuthUser.role` / `RlsConnection.role()`), NOT the api-core `AuthUser`
//! extractor. Route handlers pass `rls.role()` (or `auth.role`) in.
//!
//! Mapping to HTTP lives in accounting-server: a handler turns
//! `Err(AccountingError::Unauthorized(_))` into `StatusCode::FORBIDDEN`.
//!
//! ## Design (UC-ACC-16.2)
//! Authorization is a pure, server-side decision over the *role level* carried
//! by the request principal. The client UI state is irrelevant — a read-only
//! role attempting a mutation via the raw API is rejected here, before any
//! repository call. RLS (the per-tenant DB policy) is defense-in-depth: even if
//! a check were bypassed, the `tenant_id = get_current_org_id()` policy still
//! blocks cross-company leakage. The two layers are independent on purpose.
//!
//! Role hierarchy is the single source of truth in `common::TenantRole::level()`
//! (SuperAdmin=100 … Guest=10). We never hard-code a mirror of the variant set
//! here — we compare levels and delegate admin classification to the enum — so a
//! new role variant is governed automatically.

use crate::errors::{AccountingError, Result};
use common::TenantRole;

/// Require that `actor` holds at least `needed` (by role level). Super-admin /
/// platform-admin / org-admin always pass (they administer the company).
/// Returns `AccountingError::Unauthorized` otherwise.
///
/// This is THE call every mutating/reading accounting handler makes first.
/// (UC-ACC-16.2: server-side check; denied → 403 regardless of UI.)
pub fn require_role(actor: TenantRole, needed: TenantRole) -> Result<()> {
    // Admins administer the company and bypass the level gate (still RLS-scoped
    // to their active org — defense-in-depth handles cross-company isolation).
    if is_admin(actor) {
        return Ok(());
    }
    if actor.level() >= needed.level() {
        Ok(())
    } else {
        Err(AccountingError::Unauthorized(format!(
            "role {actor:?} (level {}) is below required {needed:?} (level {})",
            actor.level(),
            needed.level()
        )))
    }
}

/// Require that an OPTIONAL actor role is present and satisfies `needed`. Useful
/// where the role comes from `AuthUser.role: Option<TenantRole>` (a principal
/// with no tenant membership has `None` and must be denied — UC-ACC-16.2).
pub fn require_role_opt(actor: Option<TenantRole>, needed: TenantRole) -> Result<()> {
    match actor {
        Some(role) => require_role(role, needed),
        None => Err(AccountingError::Unauthorized(
            "no tenant role on principal (not a member of the active company)".to_string(),
        )),
    }
}

/// Whether the role is a platform/super/org admin (bypasses tenant-role level
/// checks within the active company). Delegates to `TenantRole::is_admin` so the
/// admin set cannot drift from the canonical definition in `common`.
pub fn is_admin(actor: TenantRole) -> bool {
    actor.is_admin()
}

/// The minimum role allowed to mutate accounting documents/settings in a
/// company. Handlers pass this to `require_role`. ACC MVP: Manager-level and up
/// (mirrors the embedded MVP's manager-only gate in api-server).
pub const MUTATE_MIN_ROLE: TenantRole = TenantRole::Manager;

/// The minimum role allowed to read accounting data in a company.
pub const READ_MIN_ROLE: TenantRole = TenantRole::Manager;

/// The minimum role allowed to view the audit log / administer security
/// (2FA policy, share-link revocation across users, DSAR/export). These are
/// company-administration concerns, so we require admin-level. (UC-ACC-16.2/.6/.7)
pub const ADMIN_MIN_ROLE: TenantRole = TenantRole::OrgAdmin;

/// Require company-administration privilege (audit log, GDPR, exports).
/// Admins pass via `is_admin`; everyone else is denied. (UC-ACC-16.6/.7)
pub fn require_admin(actor: TenantRole) -> Result<()> {
    if is_admin(actor) {
        Ok(())
    } else {
        Err(AccountingError::Unauthorized(format!(
            "role {actor:?} is not a company administrator"
        )))
    }
}

/// Require company-administration privilege from an OPTIONAL role.
pub fn require_admin_opt(actor: Option<TenantRole>) -> Result<()> {
    match actor {
        Some(role) => require_admin(role),
        None => Err(AccountingError::Unauthorized(
            "no tenant role on principal".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // UC-ACC-16.2: a sufficiently-privileged role passes the gate.
    #[test]
    fn manager_can_mutate() {
        assert!(require_role(TenantRole::Manager, MUTATE_MIN_ROLE).is_ok());
    }

    // UC-ACC-16.2: read-only / low-privilege role attempting a mutation is
    // rejected server-side (direct-API privilege escalation blocked).
    #[test]
    fn resident_cannot_mutate() {
        let err = require_role(TenantRole::Resident, MUTATE_MIN_ROLE).unwrap_err();
        assert!(matches!(err, AccountingError::Unauthorized(_)));
    }

    #[test]
    fn tenant_cannot_read_accounting() {
        assert!(require_role(TenantRole::Tenant, READ_MIN_ROLE).is_err());
    }

    // Admins bypass the level gate.
    #[test]
    fn admins_bypass_level_gate() {
        for role in [
            TenantRole::SuperAdmin,
            TenantRole::PlatformAdmin,
            TenantRole::OrgAdmin,
        ] {
            assert!(require_role(role, MUTATE_MIN_ROLE).is_ok());
            assert!(is_admin(role));
            assert!(require_admin(role).is_ok());
        }
    }

    // A non-admin manager may mutate but may NOT view the audit log / GDPR.
    #[test]
    fn manager_is_not_admin() {
        assert!(!is_admin(TenantRole::Manager));
        assert!(require_admin(TenantRole::Manager).is_err());
    }

    // UC-ACC-16.2: a principal with no tenant membership is denied.
    #[test]
    fn missing_role_is_denied() {
        assert!(require_role_opt(None, READ_MIN_ROLE).is_err());
        assert!(require_admin_opt(None).is_err());
        assert!(require_role_opt(Some(TenantRole::Manager), READ_MIN_ROLE).is_ok());
    }

    // Equal level is allowed (>=, not >).
    #[test]
    fn equal_level_allowed() {
        assert!(require_role(TenantRole::Manager, TenantRole::Manager).is_ok());
    }
}

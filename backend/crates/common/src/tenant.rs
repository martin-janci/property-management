//! Multi-tenancy context and types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Tenant context - required for all multi-tenant operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantContext {
    /// Tenant/Organization ID
    pub tenant_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// User's role in this tenant
    pub role: TenantRole,
}

impl TenantContext {
    pub fn new(tenant_id: Uuid, user_id: Uuid, role: TenantRole) -> Self {
        Self {
            tenant_id,
            user_id,
            role,
        }
    }

    /// Check if user has at least the specified role level.
    pub fn has_role(&self, required: TenantRole) -> bool {
        self.role.level() >= required.level()
    }

    /// Check if user can access a specific building based on role.
    ///
    /// This performs a synchronous role-based check. For complete building access
    /// verification (including unit ownership/residency), use the async method
    /// `BuildingRepository::can_user_access_building()` which performs database queries.
    ///
    /// # Access Rules (synchronous check only)
    /// - SuperAdmin, PlatformAdmin, OrgAdmin: Full access to all buildings in org
    /// - Manager, TechnicalManager: Full access to all buildings in org
    /// - Other roles: Require async database check via BuildingRepository
    ///
    /// # Returns
    /// - `Some(true)` if role grants unconditional access
    /// - `Some(false)` if role denies access (Guest role)
    /// - `None` if access cannot be determined without database query
    pub fn can_access_building_by_role(&self, _building_id: Uuid) -> Option<bool> {
        match self.role {
            // Admin and manager roles have access to all buildings in the organization
            TenantRole::SuperAdmin
            | TenantRole::PlatformAdmin
            | TenantRole::OrgAdmin
            | TenantRole::Manager
            | TenantRole::TechnicalManager => Some(true),
            // Guest role has no building access
            TenantRole::Guest => Some(false),
            // Other roles (Owner, Tenant, Resident, etc.) require database check
            // to verify unit ownership/residency in the specific building
            _ => None,
        }
    }

    /// Check if user can access a specific building (legacy method).
    ///
    /// **DEPRECATED**: This method always returns `true` for backward compatibility.
    /// Use `can_access_building_by_role()` for synchronous role-based checks, or
    /// `BuildingRepository::can_user_access_building()` for complete access verification.
    #[deprecated(
        since = "0.2.207",
        note = "Use can_access_building_by_role() or BuildingRepository::can_user_access_building()"
    )]
    pub fn can_access_building(&self, _building_id: Uuid) -> bool {
        // For backward compatibility, delegate to role-based check
        // Returns true if role grants access or if undetermined (requires DB check)
        self.can_access_building_by_role(_building_id)
            .unwrap_or(true)
    }
}

/// User role within tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TenantRole {
    /// Super administrator (platform level)
    SuperAdmin,
    /// Platform administrator (infrastructure/operations)
    PlatformAdmin,
    /// Organization administrator
    OrgAdmin,
    /// Building manager
    Manager,
    /// Technical manager
    TechnicalManager,
    /// Property owner
    Owner,
    /// Owner's delegate
    OwnerDelegate,
    /// Tenant/renter
    Tenant,
    /// Resident (no ownership)
    Resident,
    /// Short-term rental property manager
    PropertyManager,
    /// Real estate agent
    RealEstateAgent,
    /// Guest (temporary access)
    Guest,
}

impl TenantRole {
    /// Get role hierarchy level (higher = more permissions).
    pub fn level(&self) -> u8 {
        match self {
            TenantRole::SuperAdmin => 100,
            TenantRole::PlatformAdmin => 95,
            TenantRole::OrgAdmin => 90,
            TenantRole::Manager => 80,
            TenantRole::TechnicalManager => 75,
            TenantRole::Owner => 60,
            TenantRole::OwnerDelegate => 55,
            TenantRole::PropertyManager => 50,
            TenantRole::RealEstateAgent => 45,
            TenantRole::Tenant => 40,
            TenantRole::Resident => 30,
            TenantRole::Guest => 10,
        }
    }

    /// Check if role is admin-level.
    pub fn is_admin(&self) -> bool {
        matches!(
            self,
            TenantRole::SuperAdmin | TenantRole::PlatformAdmin | TenantRole::OrgAdmin
        )
    }

    /// Check if role is manager-level.
    pub fn is_manager(&self) -> bool {
        matches!(
            self,
            TenantRole::SuperAdmin
                | TenantRole::PlatformAdmin
                | TenantRole::OrgAdmin
                | TenantRole::Manager
                | TenantRole::TechnicalManager
        )
    }

    /// Parse the canonical snake_case `role_type` string — as stored in
    /// `organization_members.role_type` and serialized in JWT claims — into a
    /// [`TenantRole`]. Returns `None` for an unknown/legacy value.
    ///
    /// Reuses the same serde `rename_all = "snake_case"` mapping as
    /// serialization, so it cannot drift from the wire/DB representation: a new
    /// variant becomes parseable the moment it is added to the enum.
    /// Authorization gates should derive their decision from
    /// [`is_manager`](Self::is_manager) via this parser rather than hard-coding a
    /// mirror of the variant strings.
    pub fn from_role_type(role_type: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(role_type.to_owned())).ok()
    }
}

impl std::fmt::Display for TenantRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantRole::SuperAdmin => write!(f, "Super Admin"),
            TenantRole::PlatformAdmin => write!(f, "Platform Admin"),
            TenantRole::OrgAdmin => write!(f, "Organization Admin"),
            TenantRole::Manager => write!(f, "Manager"),
            TenantRole::TechnicalManager => write!(f, "Technical Manager"),
            TenantRole::Owner => write!(f, "Owner"),
            TenantRole::OwnerDelegate => write!(f, "Owner Delegate"),
            TenantRole::Tenant => write!(f, "Tenant"),
            TenantRole::Resident => write!(f, "Resident"),
            TenantRole::PropertyManager => write!(f, "Property Manager"),
            TenantRole::RealEstateAgent => write!(f, "Real Estate Agent"),
            TenantRole::Guest => write!(f, "Guest"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(role: TenantRole) -> TenantContext {
        TenantContext::new(Uuid::new_v4(), Uuid::new_v4(), role)
    }

    // ---- TenantRole::level ordering ----

    #[test]
    fn role_levels_form_a_strict_hierarchy() {
        // Higher numbers = more privileges. Pin the ordering so any change
        // to the level() function is caught.
        assert!(TenantRole::SuperAdmin.level() > TenantRole::PlatformAdmin.level());
        assert!(TenantRole::PlatformAdmin.level() > TenantRole::OrgAdmin.level());
        assert!(TenantRole::OrgAdmin.level() > TenantRole::Manager.level());
        assert!(TenantRole::Manager.level() > TenantRole::TechnicalManager.level());
        assert!(TenantRole::TechnicalManager.level() > TenantRole::Owner.level());
        assert!(TenantRole::Owner.level() > TenantRole::OwnerDelegate.level());
        assert!(TenantRole::OwnerDelegate.level() > TenantRole::PropertyManager.level());
        assert!(TenantRole::PropertyManager.level() > TenantRole::RealEstateAgent.level());
        assert!(TenantRole::RealEstateAgent.level() > TenantRole::Tenant.level());
        assert!(TenantRole::Tenant.level() > TenantRole::Resident.level());
        assert!(TenantRole::Resident.level() > TenantRole::Guest.level());
    }

    // ---- is_admin / is_manager classification ----

    #[test]
    fn is_admin_includes_only_super_platform_org() {
        assert!(TenantRole::SuperAdmin.is_admin());
        assert!(TenantRole::PlatformAdmin.is_admin());
        assert!(TenantRole::OrgAdmin.is_admin());

        assert!(!TenantRole::Manager.is_admin());
        assert!(!TenantRole::TechnicalManager.is_admin());
        assert!(!TenantRole::Owner.is_admin());
        assert!(!TenantRole::Tenant.is_admin());
        assert!(!TenantRole::Guest.is_admin());
    }

    #[test]
    fn is_manager_includes_admins_and_managers() {
        assert!(TenantRole::SuperAdmin.is_manager());
        assert!(TenantRole::PlatformAdmin.is_manager());
        assert!(TenantRole::OrgAdmin.is_manager());
        assert!(TenantRole::Manager.is_manager());
        assert!(TenantRole::TechnicalManager.is_manager());

        assert!(!TenantRole::Owner.is_manager());
        assert!(!TenantRole::OwnerDelegate.is_manager());
        assert!(!TenantRole::PropertyManager.is_manager());
        assert!(!TenantRole::Tenant.is_manager());
        assert!(!TenantRole::Resident.is_manager());
        assert!(!TenantRole::Guest.is_manager());
    }

    // ---- TenantContext::has_role ----

    #[test]
    fn has_role_passes_for_higher_or_equal_levels() {
        let ctx = ctx(TenantRole::OrgAdmin);
        assert!(ctx.has_role(TenantRole::Guest));
        assert!(ctx.has_role(TenantRole::Owner));
        assert!(ctx.has_role(TenantRole::Manager));
        assert!(ctx.has_role(TenantRole::OrgAdmin));
        // Same level is allowed.
        assert!(ctx.has_role(TenantRole::OrgAdmin));
    }

    #[test]
    fn has_role_fails_for_higher_required_levels() {
        let ctx = ctx(TenantRole::Tenant);
        assert!(!ctx.has_role(TenantRole::Manager));
        assert!(!ctx.has_role(TenantRole::OrgAdmin));
        assert!(!ctx.has_role(TenantRole::SuperAdmin));
    }

    #[test]
    fn has_role_guest_only_passes_for_guest() {
        let ctx = ctx(TenantRole::Guest);
        assert!(ctx.has_role(TenantRole::Guest));
        assert!(!ctx.has_role(TenantRole::Resident));
        assert!(!ctx.has_role(TenantRole::Tenant));
    }

    // ---- TenantContext::can_access_building_by_role ----

    #[test]
    fn admin_and_manager_roles_can_always_access_buildings() {
        let building_id = Uuid::new_v4();
        for role in [
            TenantRole::SuperAdmin,
            TenantRole::PlatformAdmin,
            TenantRole::OrgAdmin,
            TenantRole::Manager,
            TenantRole::TechnicalManager,
        ] {
            let ctx = ctx(role);
            assert_eq!(
                ctx.can_access_building_by_role(building_id),
                Some(true),
                "role {:?} should always grant building access",
                role
            );
        }
    }

    #[test]
    fn guest_role_is_explicitly_denied() {
        let ctx = ctx(TenantRole::Guest);
        assert_eq!(ctx.can_access_building_by_role(Uuid::new_v4()), Some(false));
    }

    #[test]
    fn other_roles_require_database_check() {
        for role in [
            TenantRole::Owner,
            TenantRole::OwnerDelegate,
            TenantRole::Tenant,
            TenantRole::Resident,
            TenantRole::PropertyManager,
            TenantRole::RealEstateAgent,
        ] {
            let ctx = ctx(role);
            assert_eq!(
                ctx.can_access_building_by_role(Uuid::new_v4()),
                None,
                "role {:?} should require an async DB check",
                role
            );
        }
    }

    // ---- TenantContext::new wiring ----

    #[test]
    fn new_preserves_ids_and_role() {
        let tenant = Uuid::new_v4();
        let user = Uuid::new_v4();
        let ctx = TenantContext::new(tenant, user, TenantRole::Manager);

        assert_eq!(ctx.tenant_id, tenant);
        assert_eq!(ctx.user_id, user);
        assert_eq!(ctx.role, TenantRole::Manager);
    }

    // ---- Display ----

    #[test]
    fn role_display_uses_human_readable_names() {
        assert_eq!(TenantRole::SuperAdmin.to_string(), "Super Admin");
        assert_eq!(TenantRole::PlatformAdmin.to_string(), "Platform Admin");
        assert_eq!(TenantRole::OrgAdmin.to_string(), "Organization Admin");
        assert_eq!(TenantRole::Manager.to_string(), "Manager");
        assert_eq!(
            TenantRole::TechnicalManager.to_string(),
            "Technical Manager"
        );
        assert_eq!(TenantRole::Owner.to_string(), "Owner");
        assert_eq!(TenantRole::OwnerDelegate.to_string(), "Owner Delegate");
        assert_eq!(TenantRole::Tenant.to_string(), "Tenant");
        assert_eq!(TenantRole::Resident.to_string(), "Resident");
        assert_eq!(TenantRole::PropertyManager.to_string(), "Property Manager");
        assert_eq!(TenantRole::RealEstateAgent.to_string(), "Real Estate Agent");
        assert_eq!(TenantRole::Guest.to_string(), "Guest");
    }

    // ---- Serde JSON contract ----

    #[test]
    fn from_role_type_round_trips_snake_case_and_drives_is_manager() {
        // The canonical role_type strings stored in organization_members /
        // serialized in JWT claims must parse back to the right variant.
        for role in [
            TenantRole::SuperAdmin,
            TenantRole::PlatformAdmin,
            TenantRole::OrgAdmin,
            TenantRole::Manager,
            TenantRole::TechnicalManager,
            TenantRole::Owner,
            TenantRole::OwnerDelegate,
            TenantRole::Tenant,
            TenantRole::Resident,
            TenantRole::PropertyManager,
            TenantRole::RealEstateAgent,
            TenantRole::Guest,
        ] {
            let s = serde_json::to_value(role).unwrap();
            let s = s.as_str().unwrap();
            assert_eq!(
                TenantRole::from_role_type(s),
                Some(role),
                "role_type string {s:?} must parse back to its variant"
            );
        }

        // The exact manager set the OTA gate depends on (#1525/#1585).
        for s in [
            "super_admin",
            "platform_admin",
            "org_admin",
            "manager",
            "technical_manager",
        ] {
            assert!(
                TenantRole::from_role_type(s).is_some_and(|r| r.is_manager()),
                "{s} must parse and be a manager role"
            );
        }
        for s in ["owner", "tenant", "resident", "guest", "real_estate_agent"] {
            assert!(
                TenantRole::from_role_type(s).is_some_and(|r| !r.is_manager()),
                "{s} must parse and NOT be a manager role"
            );
        }

        // Unknown/legacy values are rejected (gate fails closed → 403).
        assert_eq!(TenantRole::from_role_type("Manager"), None);
        assert_eq!(TenantRole::from_role_type("not_a_role"), None);
        assert_eq!(TenantRole::from_role_type(""), None);
    }

    #[test]
    fn role_serializes_to_snake_case() {
        // The enum is annotated with #[serde(rename_all = "snake_case")];
        // pin that contract because JWT claims rely on it.
        assert_eq!(
            serde_json::to_string(&TenantRole::OrgAdmin).unwrap(),
            "\"org_admin\""
        );
        assert_eq!(
            serde_json::to_string(&TenantRole::TechnicalManager).unwrap(),
            "\"technical_manager\""
        );
        assert_eq!(
            serde_json::to_string(&TenantRole::OwnerDelegate).unwrap(),
            "\"owner_delegate\""
        );
        assert_eq!(
            serde_json::to_string(&TenantRole::PropertyManager).unwrap(),
            "\"property_manager\""
        );
        assert_eq!(
            serde_json::to_string(&TenantRole::SuperAdmin).unwrap(),
            "\"super_admin\""
        );
    }

    #[test]
    fn role_round_trips_through_json() {
        for role in [
            TenantRole::SuperAdmin,
            TenantRole::PlatformAdmin,
            TenantRole::OrgAdmin,
            TenantRole::Manager,
            TenantRole::TechnicalManager,
            TenantRole::Owner,
            TenantRole::OwnerDelegate,
            TenantRole::Tenant,
            TenantRole::Resident,
            TenantRole::PropertyManager,
            TenantRole::RealEstateAgent,
            TenantRole::Guest,
        ] {
            let encoded = serde_json::to_string(&role).unwrap();
            let decoded: TenantRole = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, role);
        }
    }

    #[test]
    fn tenant_context_round_trips_through_json() {
        let ctx = TenantContext::new(Uuid::nil(), Uuid::nil(), TenantRole::Manager);
        let encoded = serde_json::to_string(&ctx).unwrap();
        // Field names in the JSON should match what handlers like
        // api-server's `extract_tenant_context` rely on.
        assert!(encoded.contains("\"tenant_id\""));
        assert!(encoded.contains("\"user_id\""));
        assert!(encoded.contains("\"role\":\"manager\""));

        let decoded: TenantContext = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.tenant_id, ctx.tenant_id);
        assert_eq!(decoded.user_id, ctx.user_id);
        assert_eq!(decoded.role, ctx.role);
    }
}

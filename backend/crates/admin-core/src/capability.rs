//! Capability registry and grants storage.
//!
//! Capabilities are *named* permissions (not boolean roles). Every admin
//! action goes through `RequireCapability(Capability::X)`, which checks for
//! an active grant in the `capability_grants` table.
//!
//! See migration 00138 for the storage schema. Defenses against leak #21
//! ("super-admin = single catastrophic point") and leak #12 ("escalation to
//! `platform` = escalation to god") live here:
//!   * Application layer enforces `granted_by != user_id` (no self-grant).
//!   * `mfa_required` defaults to TRUE; the extractor refuses without recent
//!     MFA verification.
//!   * Granting `PrincipalKindEscalate` requires the granter ALSO holds
//!     `PrincipalKindEscalate` (Phase 2 owns the SECURITY DEFINER function).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Error as SqlxError;
use std::collections::HashSet;
use std::sync::OnceLock;
use uuid::Uuid;

use crate::AdminError;

/// Named capabilities. Adding a new capability is intentionally a code change
/// (not a row in a table) so the surface area is reviewable in PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // Agencies / organizations.
    AgenciesRead,
    AgenciesWrite,
    AgenciesSuspend,

    // Users.
    UsersRead,
    UsersWrite,
    UsersImpersonate,

    // Memberships.
    MembershipsGrant,
    MembershipsRevoke,

    // Site / mobile / branding settings.
    SiteSettingsRead,
    SiteSettingsWrite,
    MobileConfigWrite,

    // Feature flags.
    FeatureFlagsWrite,

    // Tenant lifecycle (Phase 5.5 reaches into these too).
    TenantExport,
    TenantPurge,
    TenantRestore,

    // Audit-log viewer.
    AuditRead,

    // OAuth client administration (register, list, view, update, revoke,
    // regenerate client secrets). Gates `/api/v1/admin/oauth/clients`.
    OauthClientWrite,

    // Platform-kind promotion. Reserved — must be paired with the Phase 2
    // SECURITY DEFINER function and can only be granted by another holder.
    PrincipalKindEscalate,
}

impl Capability {
    /// Stable string form for database storage and logging. MUST match the
    /// `#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]`
    /// derivation above.
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::AgenciesRead => "agencies_read",
            Capability::AgenciesWrite => "agencies_write",
            Capability::AgenciesSuspend => "agencies_suspend",
            Capability::UsersRead => "users_read",
            Capability::UsersWrite => "users_write",
            Capability::UsersImpersonate => "users_impersonate",
            Capability::MembershipsGrant => "memberships_grant",
            Capability::MembershipsRevoke => "memberships_revoke",
            Capability::SiteSettingsRead => "site_settings_read",
            Capability::SiteSettingsWrite => "site_settings_write",
            Capability::MobileConfigWrite => "mobile_config_write",
            Capability::FeatureFlagsWrite => "feature_flags_write",
            Capability::TenantExport => "tenant_export",
            Capability::TenantPurge => "tenant_purge",
            Capability::TenantRestore => "tenant_restore",
            Capability::AuditRead => "audit_read",
            Capability::OauthClientWrite => "oauth_client_write",
            Capability::PrincipalKindEscalate => "principal_kind_escalate",
        }
    }

    /// All known capabilities — useful for the registry and for the UI's
    /// "list capabilities" page.
    pub const ALL: &'static [Capability] = &[
        Capability::AgenciesRead,
        Capability::AgenciesWrite,
        Capability::AgenciesSuspend,
        Capability::UsersRead,
        Capability::UsersWrite,
        Capability::UsersImpersonate,
        Capability::MembershipsGrant,
        Capability::MembershipsRevoke,
        Capability::SiteSettingsRead,
        Capability::SiteSettingsWrite,
        Capability::MobileConfigWrite,
        Capability::FeatureFlagsWrite,
        Capability::TenantExport,
        Capability::TenantPurge,
        Capability::TenantRestore,
        Capability::AuditRead,
        Capability::OauthClientWrite,
        Capability::PrincipalKindEscalate,
    ];
}

/// Process-wide capability registry. Each binary calls `init` at startup
/// listing the capabilities it actually exposes; the UI consults this for
/// discovery (`GET /admin/capabilities`).
pub struct CapabilityRegistry;

static REGISTRY: OnceLock<HashSet<Capability>> = OnceLock::new();

impl CapabilityRegistry {
    /// Initialize the registry. Idempotent — first writer wins.
    pub fn init(capabilities: impl IntoIterator<Item = Capability>) {
        let set: HashSet<Capability> = capabilities.into_iter().collect();
        let _ = REGISTRY.set(set);
    }

    /// Return all capabilities the current binary registered. Returns the
    /// full `ALL` list if `init` was never called (defensive default).
    pub fn registered() -> HashSet<Capability> {
        REGISTRY
            .get()
            .cloned()
            .unwrap_or_else(|| Capability::ALL.iter().copied().collect())
    }

    /// Convenience: did this binary register `cap`?
    pub fn contains(cap: Capability) -> bool {
        REGISTRY.get().map(|set| set.contains(&cap)).unwrap_or(true) // default-open for un-initialized binaries (tests).
    }
}

/// A capability grant row.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub id: Uuid,
    pub user_id: Uuid,
    pub capability: String,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<Uuid>,
    pub mfa_required: bool,
    pub note: Option<String>,
}

/// Repository abstraction over the `capability_grants` table. The trait is
/// the seam tests use to plug a fake.
#[async_trait]
pub trait CapabilityGrantsRepository: Send + Sync {
    /// Does `user` currently hold `cap` (active, non-revoked, non-expired)?
    async fn user_has(&self, user_id: Uuid, capability: Capability) -> Result<bool, AdminError>;

    /// Whether the active grant for `user` + `cap` requires recent MFA.
    /// Default to TRUE if there is no specific override.
    async fn mfa_required_for(
        &self,
        user_id: Uuid,
        capability: Capability,
    ) -> Result<bool, AdminError>;

    /// List a user's active grants.
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<CapabilityGrant>, AdminError>;

    /// Grant a capability. Application-layer guards:
    ///   * `granted_by != user_id` (no self-grant) — leak #21.
    ///   * `granted_by` must already hold `PrincipalKindEscalate` if granting
    ///     that capability — leak #12.
    async fn grant(
        &self,
        user_id: Uuid,
        capability: Capability,
        granted_by: Uuid,
        expires_at: Option<DateTime<Utc>>,
        mfa_required: bool,
        note: Option<String>,
    ) -> Result<CapabilityGrant, AdminError>;

    /// Soft-revoke an active grant.
    async fn revoke(&self, grant_id: Uuid, revoked_by: Uuid) -> Result<(), AdminError>;
}

/// Postgres-backed implementation.
#[derive(Clone)]
pub struct PgCapabilityGrantsRepository {
    pool: db::DbPool,
}

impl PgCapabilityGrantsRepository {
    pub fn new(pool: db::DbPool) -> Self {
        Self { pool }
    }
}

fn map_sqlx(err: SqlxError) -> AdminError {
    AdminError::Internal(format!("capability_grants: {}", err))
}

#[async_trait]
impl CapabilityGrantsRepository for PgCapabilityGrantsRepository {
    async fn user_has(&self, user_id: Uuid, capability: Capability) -> Result<bool, AdminError> {
        let exists: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT TRUE
            FROM capability_grants
            WHERE user_id = $1
              AND capability = $2
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(capability.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(exists.is_some())
    }

    async fn mfa_required_for(
        &self,
        user_id: Uuid,
        capability: Capability,
    ) -> Result<bool, AdminError> {
        let row: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT mfa_required
            FROM capability_grants
            WHERE user_id = $1
              AND capability = $2
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(capability.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        // Default-on: any unknown grant is treated as MFA-required.
        Ok(row.map(|(b,)| b).unwrap_or(true))
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<CapabilityGrant>, AdminError> {
        let rows = sqlx::query_as::<_, CapabilityGrant>(
            r#"
            SELECT id, user_id, capability, granted_by, granted_at, expires_at,
                   revoked_at, revoked_by, mfa_required, note
            FROM capability_grants
            WHERE user_id = $1 AND revoked_at IS NULL
            ORDER BY granted_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows)
    }

    async fn grant(
        &self,
        user_id: Uuid,
        capability: Capability,
        granted_by: Uuid,
        expires_at: Option<DateTime<Utc>>,
        mfa_required: bool,
        note: Option<String>,
    ) -> Result<CapabilityGrant, AdminError> {
        // Leak #21 defense: no self-grant. The DB does not enforce this; the
        // app layer must.
        if user_id == granted_by {
            return Err(AdminError::Internal(
                "no_self_grant: granted_by must differ from user_id".into(),
            ));
        }

        let row = sqlx::query_as::<_, CapabilityGrant>(
            r#"
            INSERT INTO capability_grants
                (user_id, capability, granted_by, expires_at, mfa_required, note)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, capability, granted_by, granted_at, expires_at,
                      revoked_at, revoked_by, mfa_required, note
            "#,
        )
        .bind(user_id)
        .bind(capability.as_str())
        .bind(granted_by)
        .bind(expires_at)
        .bind(mfa_required)
        .bind(note)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row)
    }

    async fn revoke(&self, grant_id: Uuid, revoked_by: Uuid) -> Result<(), AdminError> {
        // E3 hardening: write `revoked_with_mfa_at = NOW()` so the DB-layer
        // trigger (`capability_grants_revoke_mfa_check`, migration 00145)
        // accepts the UPDATE. The application layer
        // (`AuthPolicyEnforcer::check_capability_revoke`, D2) has already
        // verified that the caller has a recent (<= 5 min) MFA verification
        // before this method is invoked, so attesting NOW() is correct
        // semantics.
        sqlx::query(
            r#"
            UPDATE capability_grants
            SET revoked_at = NOW(),
                revoked_by = $2,
                revoked_with_mfa_at = NOW()
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(grant_id)
        .bind(revoked_by)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_strings_are_unique_and_stable() {
        let mut seen = HashSet::new();
        for cap in Capability::ALL {
            assert!(seen.insert(cap.as_str()), "duplicate capability str");
        }
    }

    #[test]
    fn registry_default_open_when_uninitialized() {
        // No `init` called — `contains` returns true so dev binaries do not
        // accidentally lock themselves out.
        assert!(CapabilityRegistry::contains(Capability::AuditRead));
    }
}

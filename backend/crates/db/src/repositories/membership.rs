//! User membership repository (Phase 2 — Identity Unification).
//!
//! Operates on the `user_memberships` table that backs the per-request
//! authorization model in `RequestPrincipal`. Every grant/revoke writes an
//! audit_logs row using the `account_updated`/`org_member_added`/
//! `org_member_removed` actions, so a tampered membership is always traceable
//! through the existing audit infrastructure.

use crate::models::membership::{GrantMembership, UserMembership};
use crate::DbPool;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for `user_memberships` rows.
#[derive(Clone)]
pub struct MembershipRepository {
    pool: DbPool,
}

impl MembershipRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert a new active grant. Idempotent on `(user_id, organization_id, role)`:
    /// if a row already exists with the same key, the call returns the existing
    /// row (re-grant of an already-active role is a no-op, not an error).
    ///
    /// Writes an audit_logs row tagged `org_member_added` with the actor.
    pub async fn grant(&self, data: GrantMembership) -> Result<UserMembership, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // ON CONFLICT DO UPDATE — re-arm any previously-revoked grant by clearing
        // revoked_at and refreshing granted_at/granted_by. This is the explicit
        // "re-grant" semantic the admin endpoints depend on.
        let row = sqlx::query_as::<_, UserMembership>(
            r#"
            INSERT INTO user_memberships (
                user_id, organization_id, role, granted_by, granted_at, expires_at, revoked_at
            )
            VALUES ($1, $2, $3, $4, NOW(), $5, NULL)
            ON CONFLICT (user_id, organization_id, role) DO UPDATE
                SET granted_by = EXCLUDED.granted_by,
                    granted_at = NOW(),
                    expires_at = EXCLUDED.expires_at,
                    revoked_at = NULL
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(data.organization_id)
        .bind(&data.role)
        .bind(data.granted_by)
        .bind(data.expires_at)
        .fetch_one(&mut *tx)
        .await?;

        // Audit row — same `audit_logs` table the platform_admin support flow uses.
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                user_id, action, resource_type, resource_id, org_id, details
            )
            VALUES ($1, 'org_member_added'::audit_action, 'user_memberships', $2, $3, $4)
            "#,
        )
        .bind(data.granted_by)
        .bind(data.user_id)
        .bind(data.organization_id)
        .bind(serde_json::json!({
            "role":       data.role,
            "user_id":    data.user_id,
            "expires_at": data.expires_at,
        }))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    /// Mark every (user_id, organization_id) row revoked NOW. Returns the list
    /// of roles that were revoked. Writes one audit_logs row per revocation
    /// (so each role transition is independently auditable).
    pub async fn revoke(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        revoked_by: Option<Uuid>,
    ) -> Result<Vec<String>, SqlxError> {
        let mut tx = self.pool.begin().await?;

        let revoked_roles: Vec<String> = sqlx::query_scalar(
            r#"
            UPDATE user_memberships
               SET revoked_at = NOW()
             WHERE user_id = $1
               AND organization_id = $2
               AND revoked_at IS NULL
             RETURNING role
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_all(&mut *tx)
        .await?;

        for role in &revoked_roles {
            sqlx::query(
                r#"
                INSERT INTO audit_logs (
                    user_id, action, resource_type, resource_id, org_id, details
                )
                VALUES ($1, 'org_member_removed'::audit_action, 'user_memberships', $2, $3, $4)
                "#,
            )
            .bind(revoked_by)
            .bind(user_id)
            .bind(organization_id)
            .bind(serde_json::json!({ "role": role, "user_id": user_id }))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(revoked_roles)
    }

    /// All ACTIVE rows for a user, across orgs.
    ///
    /// Rows are returned in a deterministic order (`granted_at ASC,
    /// organization_id ASC, role ASC` — a total order given the
    /// `(user_id, organization_id, role)` primary key). Union callers
    /// (`check_login`, `check_password_change`,
    /// `check_capability_grant_for_user`) are order-insensitive, but
    /// `.first()` callers (e.g. `check_principal_kind_change`'s liveness
    /// policy load) rely on this tiebreak so their behavior does not depend
    /// on Postgres's arbitrary physical row order (issue #2861).
    pub async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<UserMembership>, SqlxError> {
        sqlx::query_as::<_, UserMembership>(
            r#"
            SELECT * FROM user_memberships
            WHERE user_id = $1
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY granted_at ASC, organization_id ASC, role ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// All active member user ids for an organization (de-duplicated across
    /// roles). Used by Epic 2B fan-out (e.g. critical-notification broadcast)
    /// to resolve every recipient in an org. Mirrors `list_for_user`'s
    /// bare-pool access pattern.
    pub async fn list_active_member_ids(&self, org_id: Uuid) -> Result<Vec<Uuid>, SqlxError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id FROM user_memberships
            WHERE organization_id = $1
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Returns `true` iff the user has at least one active membership in the
    /// given org. This is the hot-path query used by `RequestPrincipal` on
    /// every request.
    pub async fn is_active(&self, user_id: Uuid, organization_id: Uuid) -> Result<bool, SqlxError> {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_memberships
                 WHERE user_id = $1
                   AND organization_id = $2
                   AND revoked_at IS NULL
                   AND (expires_at IS NULL OR expires_at > NOW())
            )
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Returns `true` iff the user has an active membership with a
    /// manager-tier role in the given org. Manager-tier roles are the same
    /// set that `TenantRole::is_manager_role` recognizes:
    ///   SuperAdmin, PlatformAdmin, OrgAdmin, Manager, TechnicalManager,
    ///   PropertyManager.
    ///
    /// Used by handlers that gate manager-only data (internal notes on
    /// faults, full timeline visibility, etc.). Previously these handlers
    /// hardcoded `is_manager = false` as a least-privilege placeholder
    /// (COMP-005 / P0-07).
    pub async fn is_manager_in_org(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_memberships
                 WHERE user_id = $1
                   AND organization_id = $2
                   AND revoked_at IS NULL
                   AND (expires_at IS NULL OR expires_at > NOW())
                   AND role IN (
                       'SuperAdmin',
                       'PlatformAdmin',
                       'OrgAdmin',
                       'Manager',
                       'TechnicalManager',
                       'PropertyManager'
                   )
            )
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Return the user IDs of all active manager-tier members in `org_id`.
    ///
    /// Manager-tier roles are the same set as [`is_manager_in_org`].
    pub async fn list_manager_ids(&self, org_id: Uuid) -> Result<Vec<Uuid>, SqlxError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id FROM user_memberships
            WHERE organization_id = $1
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
              AND role IN (
                  'SuperAdmin',
                  'PlatformAdmin',
                  'OrgAdmin',
                  'Manager',
                  'TechnicalManager',
                  'PropertyManager'
              )
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

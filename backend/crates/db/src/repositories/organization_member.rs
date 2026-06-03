//! Organization member repository (Epic 2A, Story 2A.5).
//!
//! # RLS Integration
//!
//! This repository supports two usage patterns:
//!
//! 1. **RLS-aware** (recommended): Use methods with `_rls` suffix that accept an executor
//!    with RLS context already set (e.g., from `RlsConnection`).
//!
//! 2. **Legacy**: Use methods without suffix that use the internal pool. These do NOT
//!    enforce RLS and should be migrated to the RLS-aware pattern.

use crate::models::organization_member::{
    CreateOrganizationMember, MembershipStatus, OrganizationMember, OrganizationMemberWithUser,
    UpdateOrganizationMember, UserOrganizationMembership,
};
use crate::DbPool;
use chrono::Utc;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for organization member operations.
#[derive(Clone)]
pub struct OrganizationMemberRepository {
    pool: DbPool,
}

impl OrganizationMemberRepository {
    /// Create a new OrganizationMemberRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    /// Find membership by organization and user with RLS context.
    pub async fn find_by_org_and_user_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<OrganizationMember>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            SELECT * FROM organization_members
            WHERE organization_id = $1 AND user_id = $2 AND status != 'removed'
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(executor)
        .await?;

        Ok(member)
    }

    /// Get all organizations a user belongs to with RLS context.
    pub async fn get_user_memberships_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<UserOrganizationMembership>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let memberships = sqlx::query_as::<_, UserOrganizationMembership>(
            r#"
            SELECT
                om.id as membership_id,
                o.id as organization_id,
                o.name as organization_name,
                o.slug as organization_slug,
                o.logo_url as organization_logo_url,
                om.role_type,
                r.name as role_name,
                om.status,
                om.joined_at
            FROM organization_members om
            INNER JOIN organizations o ON o.id = om.organization_id
            LEFT JOIN roles r ON r.id = om.role_id
            WHERE om.user_id = $1 AND om.status != 'removed' AND o.status != 'deleted'
            ORDER BY om.joined_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(executor)
        .await?;

        Ok(memberships)
    }

    /// List members of an organization with RLS context.
    pub async fn list_org_members_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        offset: i64,
        limit: i64,
        status_filter: Option<MembershipStatus>,
    ) -> Result<(Vec<OrganizationMemberWithUser>, i64), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // For RLS, we need two separate connections for count and data
        // This method should be called with a mutable reference to get conn twice
        // For now, we'll use a simpler approach with a single query that includes count
        let status_clause = if status_filter.is_some() {
            "AND om.status = $4"
        } else {
            "AND om.status != 'removed'"
        };

        // LEFT JOIN roles to fetch role_name in the same query — eliminates
        // the N+1 round-trip the route layer used to do per member.
        let data_query = format!(
            r#"
            SELECT
                om.id, om.organization_id, om.user_id, om.role_id, om.role_type,
                om.status, om.joined_at,
                u.email as user_email, u.name as user_name,
                r.name as role_name
            FROM organization_members om
            INNER JOIN users u ON u.id = om.user_id
            LEFT JOIN roles r ON r.id = om.role_id
            WHERE om.organization_id = $1 {}
            ORDER BY om.joined_at DESC NULLS LAST
            LIMIT $2 OFFSET $3
            "#,
            status_clause
        );

        let mut data_q =
            sqlx::query_as::<_, OrganizationMemberWithUser>(sqlx::AssertSqlSafe(data_query))
                .bind(org_id)
                .bind(limit)
                .bind(offset);
        if let Some(status) = &status_filter {
            data_q = data_q.bind(status.as_str());
        }
        let members = data_q.fetch_all(executor).await?;

        // For count, we use a separate query with the pool (RLS not strictly needed for count)
        let count_query = format!(
            r#"
            SELECT COUNT(*) FROM organization_members om
            WHERE om.organization_id = $1 {}
            "#,
            if status_filter.is_some() {
                "AND om.status = $2"
            } else {
                "AND om.status != 'removed'"
            }
        );

        let mut count_q =
            sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_query)).bind(org_id);
        if let Some(status) = &status_filter {
            count_q = count_q.bind(status.as_str());
        }
        let total = count_q.fetch_one(&self.pool).await?;

        Ok((members, total))
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    /// Add a member to an organization.
    pub async fn create(
        &self,
        data: CreateOrganizationMember,
    ) -> Result<OrganizationMember, SqlxError> {
        let now = Utc::now();
        let status = if data.invited_by.is_some() {
            "pending"
        } else {
            "active"
        };
        let joined_at = if status == "active" { Some(now) } else { None };

        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            INSERT INTO organization_members
                (organization_id, user_id, role_id, role_type, status, invited_by, invited_at, joined_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.user_id)
        .bind(data.role_id)
        .bind(&data.role_type)
        .bind(status)
        .bind(data.invited_by)
        .bind(if data.invited_by.is_some() {
            Some(now)
        } else {
            None
        })
        .bind(joined_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(member)
    }

    /// Find membership by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<OrganizationMember>, SqlxError> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            SELECT * FROM organization_members WHERE id = $1 AND status != 'removed'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(member)
    }

    /// Find membership by organization and user.
    pub async fn find_by_org_and_user(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<OrganizationMember>, SqlxError> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            SELECT * FROM organization_members
            WHERE organization_id = $1 AND user_id = $2 AND status != 'removed'
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(member)
    }

    /// Check if user is a member of organization.
    pub async fn is_member(&self, org_id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM organization_members
            WHERE organization_id = $1 AND user_id = $2 AND status = 'active'
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result > 0)
    }

    /// Accept pending invitation.
    pub async fn accept_invitation(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<OrganizationMember>, SqlxError> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            UPDATE organization_members
            SET status = 'active', joined_at = NOW(), updated_at = NOW()
            WHERE organization_id = $1 AND user_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(member)
    }

    /// Update membership.
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateOrganizationMember,
    ) -> Result<Option<OrganizationMember>, SqlxError> {
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 1;

        if data.role_id.is_some() {
            param_idx += 1;
            updates.push(format!("role_id = ${}", param_idx));
        }
        if data.role_type.is_some() {
            param_idx += 1;
            updates.push(format!("role_type = ${}", param_idx));
        }
        if data.status.is_some() {
            param_idx += 1;
            updates.push(format!("status = ${}", param_idx));
        }

        let query = format!(
            "UPDATE organization_members SET {} WHERE id = $1 AND status != 'removed' RETURNING *",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, OrganizationMember>(sqlx::AssertSqlSafe(query)).bind(id);

        if let Some(role_id) = data.role_id {
            q = q.bind(role_id);
        }
        if let Some(role_type) = &data.role_type {
            q = q.bind(role_type);
        }
        if let Some(status) = &data.status {
            q = q.bind(status.as_str());
        }

        let member = q.fetch_optional(&self.pool).await?;
        Ok(member)
    }

    /// Remove member from organization (soft delete).
    pub async fn remove(&self, id: Uuid) -> Result<Option<OrganizationMember>, SqlxError> {
        let member = sqlx::query_as::<_, OrganizationMember>(
            r#"
            UPDATE organization_members
            SET status = 'removed', updated_at = NOW()
            WHERE id = $1 AND status != 'removed'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(member)
    }

    /// List members of an organization.
    pub async fn list_org_members(
        &self,
        org_id: Uuid,
        offset: i64,
        limit: i64,
        status_filter: Option<MembershipStatus>,
    ) -> Result<(Vec<OrganizationMemberWithUser>, i64), SqlxError> {
        let status_clause = if status_filter.is_some() {
            "AND om.status = $4"
        } else {
            "AND om.status != 'removed'"
        };

        let count_query = format!(
            r#"
            SELECT COUNT(*) FROM organization_members om
            WHERE om.organization_id = $1 {}
            "#,
            if status_filter.is_some() {
                "AND om.status = $2"
            } else {
                "AND om.status != 'removed'"
            }
        );

        // LEFT JOIN roles to fetch role_name in the same query — eliminates
        // the N+1 round-trip the route layer used to do per member.
        let data_query = format!(
            r#"
            SELECT
                om.id, om.organization_id, om.user_id, om.role_id, om.role_type,
                om.status, om.joined_at,
                u.email as user_email, u.name as user_name,
                r.name as role_name
            FROM organization_members om
            INNER JOIN users u ON u.id = om.user_id
            LEFT JOIN roles r ON r.id = om.role_id
            WHERE om.organization_id = $1 {}
            ORDER BY om.joined_at DESC NULLS LAST
            LIMIT $2 OFFSET $3
            "#,
            status_clause
        );

        // Execute count query
        let mut count_q =
            sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_query)).bind(org_id);
        if let Some(status) = &status_filter {
            count_q = count_q.bind(status.as_str());
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_q =
            sqlx::query_as::<_, OrganizationMemberWithUser>(sqlx::AssertSqlSafe(data_query))
                .bind(org_id)
                .bind(limit)
                .bind(offset);
        if let Some(status) = &status_filter {
            data_q = data_q.bind(status.as_str());
        }
        let members = data_q.fetch_all(&self.pool).await?;

        Ok((members, total))
    }

    /// Get all organizations a user belongs to.
    pub async fn get_user_memberships(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserOrganizationMembership>, SqlxError> {
        let memberships = sqlx::query_as::<_, UserOrganizationMembership>(
            r#"
            SELECT
                om.id as membership_id,
                o.id as organization_id,
                o.name as organization_name,
                o.slug as organization_slug,
                o.logo_url as organization_logo_url,
                om.role_type,
                r.name as role_name,
                om.status,
                om.joined_at
            FROM organization_members om
            INNER JOIN organizations o ON o.id = om.organization_id
            LEFT JOIN roles r ON r.id = om.role_id
            WHERE om.user_id = $1
              AND om.status != 'removed'
              AND o.status != 'deleted'
              AND o.status != 'suspended'
            ORDER BY om.joined_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(memberships)
    }

    /// Get user's role_type in an organization.
    pub async fn get_user_role_type(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, SqlxError> {
        let role_type = sqlx::query_scalar::<_, String>(
            r#"
            SELECT role_type FROM organization_members
            WHERE organization_id = $1 AND user_id = $2 AND status = 'active'
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role_type)
    }
}

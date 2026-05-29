//! Role repository for RBAC (Epic 2A, Story 2A.6).
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

use crate::models::role::{CreateRole, Role, UpdateRole};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for role operations.
#[derive(Clone)]
pub struct RoleRepository {
    pool: DbPool,
}

impl RoleRepository {
    /// Create a new RoleRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    /// Find role by ID with RLS context.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Role>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let role = sqlx::query_as::<_, Role>(
            r#"
            SELECT * FROM roles WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(role)
    }

    /// List all roles for an organization with RLS context.
    pub async fn list_by_org_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<Role>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let roles = sqlx::query_as::<_, Role>(
            r#"
            SELECT * FROM roles
            WHERE organization_id = $1
            ORDER BY is_system DESC, name ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await?;

        Ok(roles)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    /// Create a new role.
    pub async fn create(&self, data: CreateRole) -> Result<Role, SqlxError> {
        let permissions = serde_json::to_value(&data.permissions).unwrap_or_default();

        let role = sqlx::query_as::<_, Role>(
            r#"
            INSERT INTO roles (organization_id, name, description, permissions)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&permissions)
        .fetch_one(&self.pool)
        .await?;

        Ok(role)
    }

    /// Find role by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Role>, SqlxError> {
        let role = sqlx::query_as::<_, Role>(
            r#"
            SELECT * FROM roles WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role)
    }

    /// Find role by name within an organization.
    pub async fn find_by_name(&self, org_id: Uuid, name: &str) -> Result<Option<Role>, SqlxError> {
        let role = sqlx::query_as::<_, Role>(
            r#"
            SELECT * FROM roles
            WHERE organization_id = $1 AND LOWER(name) = LOWER($2)
            "#,
        )
        .bind(org_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role)
    }

    /// Update role.
    pub async fn update(&self, id: Uuid, data: UpdateRole) -> Result<Option<Role>, SqlxError> {
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 1;

        if data.name.is_some() {
            param_idx += 1;
            updates.push(format!("name = ${}", param_idx));
        }
        if data.description.is_some() {
            param_idx += 1;
            updates.push(format!("description = ${}", param_idx));
        }
        if data.permissions.is_some() {
            param_idx += 1;
            updates.push(format!("permissions = ${}", param_idx));
        }

        // Cannot update system roles
        let query = format!(
            "UPDATE roles SET {} WHERE id = $1 AND is_system = FALSE RETURNING *",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, Role>(sqlx::AssertSqlSafe(query)).bind(id);

        if let Some(name) = &data.name {
            q = q.bind(name);
        }
        if let Some(description) = &data.description {
            q = q.bind(description);
        }
        if let Some(permissions) = &data.permissions {
            let perms = serde_json::to_value(permissions).unwrap_or_default();
            q = q.bind(perms);
        }

        let role = q.fetch_optional(&self.pool).await?;
        Ok(role)
    }

    /// Delete role (only non-system roles).
    pub async fn delete(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM roles WHERE id = $1 AND is_system = FALSE
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all roles for an organization.
    pub async fn list_by_org(&self, org_id: Uuid) -> Result<Vec<Role>, SqlxError> {
        let roles = sqlx::query_as::<_, Role>(
            r#"
            SELECT * FROM roles
            WHERE organization_id = $1
            ORDER BY is_system DESC, name ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(roles)
    }

    /// Get system roles for an organization.
    pub async fn get_system_roles(&self, org_id: Uuid) -> Result<Vec<Role>, SqlxError> {
        let roles = sqlx::query_as::<_, Role>(
            r#"
            SELECT * FROM roles
            WHERE organization_id = $1 AND is_system = TRUE
            ORDER BY name ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(roles)
    }

    /// Check if user has a specific permission.
    pub async fn user_has_permission(
        &self,
        user_id: Uuid,
        org_id: Uuid,
        permission: &str,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT user_has_permission($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(org_id)
        .bind(permission)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get role for a user in an organization.
    pub async fn get_user_role(
        &self,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Role>, SqlxError> {
        let role = sqlx::query_as::<_, Role>(
            r#"
            SELECT r.* FROM roles r
            INNER JOIN organization_members om ON om.role_id = r.id
            WHERE om.user_id = $1 AND om.organization_id = $2 AND om.status = 'active'
            "#,
        )
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role)
    }
}

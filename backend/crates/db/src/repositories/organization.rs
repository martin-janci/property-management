//! Organization repository (Epic 2A, Story 2A.1).
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
//!
//! ## Example
//!
//! ```rust,ignore
//! async fn create_organization(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateOrganizationRequest>,
//! ) -> Result<Json<Organization>> {
//!     let org = state.org_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(org))
//! }
//! ```

use crate::models::organization::{
    CreateOrganization, Organization, OrganizationSummary, UpdateOrganization,
};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for organization operations.
#[derive(Clone)]
pub struct OrganizationRepository {
    pool: DbPool,
}

impl OrganizationRepository {
    /// Create a new OrganizationRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    /// Create a new organization with RLS context.
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_rls<'e, E>(
        &self,
        executor: E,
        data: CreateOrganization,
    ) -> Result<Organization, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (name, slug, contact_email, logo_url, primary_color)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(&data.slug)
        .bind(&data.contact_email)
        .bind(&data.logo_url)
        .bind(&data.primary_color)
        .fetch_one(executor)
        .await?;

        Ok(org)
    }

    /// Find organization by ID with RLS context.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(org)
    }

    /// Find organizations by a batch of IDs with RLS context.
    ///
    /// Returns all matching, non-deleted organizations in a single query.
    /// Used to avoid N+1 round-trips when expanding a list of memberships
    /// into their full organization records. Result order is not guaranteed —
    /// callers should regroup via `HashMap<Uuid, Organization>` keyed by `id`.
    pub async fn find_by_ids_rls<'e, E>(
        &self,
        executor: E,
        ids: &[Uuid],
    ) -> Result<Vec<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE id = ANY($1) AND status != 'deleted'
            "#,
        )
        .bind(ids)
        .fetch_all(executor)
        .await?;

        Ok(orgs)
    }

    /// Find organization by slug with RLS context.
    pub async fn find_by_slug_rls<'e, E>(
        &self,
        executor: E,
        slug: &str,
    ) -> Result<Option<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations WHERE LOWER(slug) = LOWER($1) AND status != 'deleted'
            "#,
        )
        .bind(slug)
        .fetch_optional(executor)
        .await?;

        Ok(org)
    }

    /// Check if slug exists with RLS context.
    pub async fn slug_exists_rls<'e, E>(&self, executor: E, slug: &str) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM organizations
            WHERE LOWER(slug) = LOWER($1) AND status != 'deleted'
            "#,
        )
        .bind(slug)
        .fetch_one(executor)
        .await?;

        Ok(result > 0)
    }

    /// Update organization with RLS context.
    pub async fn update_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateOrganization,
    ) -> Result<Option<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Build dynamic update query
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 1;

        if data.name.is_some() {
            param_idx += 1;
            updates.push(format!("name = ${}", param_idx));
        }
        if data.contact_email.is_some() {
            param_idx += 1;
            updates.push(format!("contact_email = ${}", param_idx));
        }
        if data.logo_url.is_some() {
            param_idx += 1;
            updates.push(format!("logo_url = ${}", param_idx));
        }
        if data.primary_color.is_some() {
            param_idx += 1;
            updates.push(format!("primary_color = ${}", param_idx));
        }
        if data.settings.is_some() {
            param_idx += 1;
            updates.push(format!("settings = ${}", param_idx));
        }

        let query = format!(
            "UPDATE organizations SET {} WHERE id = $1 AND status != 'deleted' RETURNING *",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, Organization>(&query).bind(id);

        if let Some(name) = &data.name {
            q = q.bind(name);
        }
        if let Some(contact_email) = &data.contact_email {
            q = q.bind(contact_email);
        }
        if let Some(logo_url) = &data.logo_url {
            q = q.bind(logo_url);
        }
        if let Some(primary_color) = &data.primary_color {
            q = q.bind(primary_color);
        }
        if let Some(settings) = &data.settings {
            q = q.bind(settings);
        }

        let org = q.fetch_optional(executor).await?;
        Ok(org)
    }

    /// Suspend organization with RLS context.
    pub async fn suspend_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET status = 'suspended', updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(org)
    }

    /// Reactivate suspended organization with RLS context.
    pub async fn reactivate_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET status = 'active', updated_at = NOW()
            WHERE id = $1 AND status = 'suspended'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(org)
    }

    /// Soft delete organization with RLS context (archive).
    pub async fn archive_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Organization>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET status = 'deleted', updated_at = NOW()
            WHERE id = $1 AND status != 'deleted'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(org)
    }

    /// List organizations for a user (via memberships) with RLS context.
    pub async fn get_user_organizations_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<OrganizationSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let orgs = sqlx::query_as::<_, OrganizationSummary>(
            r#"
            SELECT o.id, o.name, o.slug, o.logo_url, o.status
            FROM organizations o
            INNER JOIN organization_members om ON om.organization_id = o.id
            WHERE om.user_id = $1 AND om.status = 'active' AND o.status != 'deleted'
            ORDER BY o.name
            "#,
        )
        .bind(user_id)
        .fetch_all(executor)
        .await?;

        Ok(orgs)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    /// Create a new organization.
    ///
    /// **Deprecated**: Use `create_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.274", note = "Use create_rls with RlsConnection instead")]
    pub async fn create(&self, data: CreateOrganization) -> Result<Organization, SqlxError> {
        self.create_rls(&self.pool, data).await
    }

    /// Find organization by ID.
    ///
    /// **Deprecated**: Use `find_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use find_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Organization>, SqlxError> {
        self.find_by_id_rls(&self.pool, id).await
    }

    /// Find organization by slug.
    ///
    /// **Deprecated**: Use `find_by_slug_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use find_by_slug_rls with RlsConnection instead"
    )]
    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Organization>, SqlxError> {
        self.find_by_slug_rls(&self.pool, slug).await
    }

    /// Check if slug exists.
    ///
    /// **Deprecated**: Use `slug_exists_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use slug_exists_rls with RlsConnection instead"
    )]
    pub async fn slug_exists(&self, slug: &str) -> Result<bool, SqlxError> {
        self.slug_exists_rls(&self.pool, slug).await
    }

    /// Update organization.
    ///
    /// **Deprecated**: Use `update_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.274", note = "Use update_rls with RlsConnection instead")]
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateOrganization,
    ) -> Result<Option<Organization>, SqlxError> {
        self.update_rls(&self.pool, id, data).await
    }

    /// Suspend organization.
    ///
    /// **Deprecated**: Use `suspend_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.274", note = "Use suspend_rls with RlsConnection instead")]
    pub async fn suspend(&self, id: Uuid) -> Result<Option<Organization>, SqlxError> {
        self.suspend_rls(&self.pool, id).await
    }

    /// Reactivate suspended organization.
    ///
    /// **Deprecated**: Use `reactivate_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use reactivate_rls with RlsConnection instead"
    )]
    pub async fn reactivate(&self, id: Uuid) -> Result<Option<Organization>, SqlxError> {
        self.reactivate_rls(&self.pool, id).await
    }

    /// Soft delete organization.
    ///
    /// **Deprecated**: Use `archive_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.274", note = "Use archive_rls with RlsConnection instead")]
    pub async fn soft_delete(&self, id: Uuid) -> Result<Option<Organization>, SqlxError> {
        self.archive_rls(&self.pool, id).await
    }

    /// List all organizations (admin only).
    pub async fn list(
        &self,
        offset: i64,
        limit: i64,
        status_filter: Option<&str>,
        search: Option<&str>,
    ) -> Result<(Vec<OrganizationSummary>, i64), SqlxError> {
        let mut conditions = vec!["status != 'deleted'".to_string()];

        if status_filter.is_some() {
            conditions.push("status = $3".to_string());
        }

        if search.is_some() {
            let search_idx = if status_filter.is_some() { 4 } else { 3 };
            conditions.push(format!(
                "(LOWER(name) LIKE '%' || LOWER(${}::text) || '%' OR LOWER(slug) LIKE '%' || LOWER(${}::text) || '%')",
                search_idx, search_idx
            ));
        }

        let where_clause = conditions.join(" AND ");

        let count_query = format!("SELECT COUNT(*) FROM organizations WHERE {}", where_clause);
        let data_query = format!(
            "SELECT id, name, slug, logo_url, status FROM organizations WHERE {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            where_clause
        );

        // Execute count query
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
        if let Some(status) = status_filter {
            count_q = count_q.bind(status);
        }
        if let Some(s) = search {
            count_q = count_q.bind(s);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_q = sqlx::query_as::<_, OrganizationSummary>(&data_query)
            .bind(limit)
            .bind(offset);
        if let Some(status) = status_filter {
            data_q = data_q.bind(status);
        }
        if let Some(s) = search {
            data_q = data_q.bind(s);
        }
        let orgs = data_q.fetch_all(&self.pool).await?;

        Ok((orgs, total))
    }

    /// List organizations with full details (for admin views).
    /// Returns full Organization objects instead of summaries.
    pub async fn list_full(
        &self,
        offset: i64,
        limit: i64,
        status_filter: Option<&str>,
        search: Option<&str>,
    ) -> Result<(Vec<Organization>, i64), SqlxError> {
        let mut conditions = vec!["status != 'deleted'".to_string()];

        if status_filter.is_some() {
            conditions.push("status = $3".to_string());
        }

        if search.is_some() {
            let search_idx = if status_filter.is_some() { 4 } else { 3 };
            conditions.push(format!(
                "(LOWER(name) LIKE '%' || LOWER(${}::text) || '%' OR LOWER(slug) LIKE '%' || LOWER(${}::text) || '%')",
                search_idx, search_idx
            ));
        }

        let where_clause = conditions.join(" AND ");

        let count_query = format!("SELECT COUNT(*) FROM organizations WHERE {}", where_clause);
        let data_query = format!(
            "SELECT * FROM organizations WHERE {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            where_clause
        );

        // Execute count query
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
        if let Some(status) = status_filter {
            count_q = count_q.bind(status);
        }
        if let Some(s) = search {
            count_q = count_q.bind(s);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_q = sqlx::query_as::<_, Organization>(&data_query)
            .bind(limit)
            .bind(offset);
        if let Some(status) = status_filter {
            data_q = data_q.bind(status);
        }
        if let Some(s) = search {
            data_q = data_q.bind(s);
        }
        let orgs = data_q.fetch_all(&self.pool).await?;

        Ok((orgs, total))
    }

    /// Get organizations for a user (via memberships).
    ///
    /// **Deprecated**: Use `get_user_organizations_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use get_user_organizations_rls with RlsConnection instead"
    )]
    pub async fn get_user_organizations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OrganizationSummary>, SqlxError> {
        self.get_user_organizations_rls(&self.pool, user_id).await
    }
}

//! Document operations (Stories 7A.1, 7A.3) — RLS-aware + legacy variants,
//! plus the unauthenticated share-token lookup helpers.

use super::internal::verify_password;
use super::DocumentRepository;
use crate::models::{
    access_scope, CreateDocument, Document, DocumentListQuery, DocumentShare, DocumentSummary,
    DocumentWithDetails, LogShareAccess, MoveDocument, ShareAccessLog, UpdateDocument,
};
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

impl DocumentRepository {
    // ========================================================================
    // RLS-aware Document Operations (Story 7A.1)
    // ========================================================================

    /// Create a new document with RLS context.
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_rls<'e, E>(
        &self,
        executor: E,
        data: CreateDocument,
    ) -> Result<Document, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let access_target_ids =
            serde_json::to_value(data.access_target_ids.unwrap_or_default()).unwrap();
        let access_roles = serde_json::to_value(data.access_roles.unwrap_or_default()).unwrap();

        sqlx::query_as::<_, Document>(
            r#"
            INSERT INTO documents (
                organization_id, folder_id, title, description, category,
                file_key, file_name, mime_type, size_bytes,
                access_scope, access_target_ids, access_roles, created_by
            )
            VALUES ($1, $2, $3, $4, $5::document_category, $6, $7, $8, $9, $10::document_access_scope, $11, $12, $13)
            RETURNING
                id,
                organization_id,
                folder_id,
                title,
                description,
                category::text AS category,
                file_key,
                file_name,
                mime_type,
                size_bytes,
                access_scope::text AS access_scope,
                access_target_ids,
                access_roles,
                created_by,
                created_at,
                updated_at,
                deleted_at,
                version_number,
                parent_document_id,
                is_current_version,
                template_id,
                generation_metadata
            "#,
        )
        .bind(data.organization_id)
        .bind(data.folder_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.category)
        .bind(&data.file_key)
        .bind(&data.file_name)
        .bind(&data.mime_type)
        .bind(data.size_bytes)
        .bind(
            data.access_scope
                .as_deref()
                .unwrap_or(access_scope::ORGANIZATION),
        )
        .bind(&access_target_ids)
        .bind(&access_roles)
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Check whether a live (non-deleted) document row references `file_key`
    /// within the given org, with RLS context (#2573).
    ///
    /// Used by the direct-upload orphan-cleanup route to refuse deleting a
    /// storage object whose bytes are still referenced by a registered
    /// document. Soft-deleted rows (`deleted_at IS NOT NULL`) do not count as
    /// live references.
    pub async fn exists_by_file_key_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        file_key: &str,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM documents
                WHERE organization_id = $1
                  AND file_key = $2
                  AND deleted_at IS NULL
            ) AS "exists"
            "#,
        )
        .bind(org_id)
        .bind(file_key)
        .fetch_one(executor)
        .await?;

        Ok(row.get("exists"))
    }

    /// Find document by ID with RLS context.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Document>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Document>(
            r#"
            SELECT
                id, organization_id, folder_id, title, description,
                category::text AS category, file_key, file_name, mime_type,
                size_bytes, access_scope::text AS access_scope,
                access_target_ids, access_roles, created_by, created_at,
                updated_at, deleted_at, version_number, parent_document_id,
                is_current_version, template_id, generation_metadata
            FROM documents
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Find document by ID with details with RLS context.
    pub async fn find_by_id_with_details_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<DocumentWithDetails>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT
                d.*,
                d.category::text AS category_text,
                d.access_scope::text AS access_scope_text,
                u.name as created_by_name,
                f.name as folder_name,
                COALESCE(s.share_count, 0) as share_count
            FROM documents d
            LEFT JOIN users u ON u.id = d.created_by
            LEFT JOIN document_folders f ON f.id = d.folder_id
            LEFT JOIN (
                SELECT document_id, COUNT(*) as share_count
                FROM document_shares
                WHERE revoked_at IS NULL
                GROUP BY document_id
            ) s ON s.document_id = d.id
            WHERE d.id = $1 AND d.deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|r| DocumentWithDetails {
            document: Document {
                id: r.get("id"),
                organization_id: r.get("organization_id"),
                folder_id: r.get("folder_id"),
                title: r.get("title"),
                description: r.get("description"),
                // `category` / `access_scope` are PG enums; `d.*` returns them
                // raw, which fails a `String` decode (#1008). Read the ::text
                // aliases instead (matches the sibling find methods).
                category: r.get("category_text"),
                file_key: r.get("file_key"),
                file_name: r.get("file_name"),
                mime_type: r.get("mime_type"),
                size_bytes: r.get("size_bytes"),
                access_scope: r.get("access_scope_text"),
                access_target_ids: r.get("access_target_ids"),
                access_roles: r.get("access_roles"),
                created_by: r.get("created_by"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                deleted_at: r.get("deleted_at"),
                version_number: r.get("version_number"),
                parent_document_id: r.get("parent_document_id"),
                is_current_version: r.get("is_current_version"),
                template_id: r.get("template_id"),
                generation_metadata: r.get("generation_metadata"),
            },
            created_by_name: r.get("created_by_name"),
            folder_name: r.get("folder_name"),
            share_count: r.get("share_count"),
        }))
    }

    /// List documents for an organization with filters with RLS context.
    pub async fn list_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: DocumentListQuery,
    ) -> Result<Vec<DocumentSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, DocumentSummary>(
            r#"
            SELECT
                id, title, category::text AS category, file_name, mime_type, size_bytes, folder_id, created_at
            FROM documents
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND ($2::uuid IS NULL OR folder_id = $2)
              AND ($3::text IS NULL OR category = $3::document_category)
              AND ($4::uuid IS NULL OR created_by = $4)
              AND ($5::text IS NULL OR title ILIKE '%' || $5 || '%')
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(query.folder_id)
        .bind(&query.category)
        .bind(query.created_by)
        .bind(&query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Count documents matching query with RLS context.
    pub async fn count_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: DocumentListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM documents
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND ($2::uuid IS NULL OR folder_id = $2)
              AND ($3::text IS NULL OR category = $3::document_category)
              AND ($4::uuid IS NULL OR created_by = $4)
              AND ($5::text IS NULL OR title ILIKE '%' || $5 || '%')
            "#,
        )
        .bind(org_id)
        .bind(query.folder_id)
        .bind(&query.category)
        .bind(query.created_by)
        .bind(&query.search)
        .fetch_one(executor)
        .await?;

        Ok(row.get("count"))
    }

    /// List documents accessible by a specific user with RLS context (Story 7A.3).
    #[allow(clippy::too_many_arguments)]
    pub async fn list_accessible_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        user_building_ids: &[Uuid],
        user_unit_ids: &[Uuid],
        user_roles: &[String],
        query: DocumentListQuery,
    ) -> Result<Vec<DocumentSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        // `?|` is `jsonb ?| text[]` — right operand must be text[], not jsonb.
        let building_ids: Vec<String> = user_building_ids.iter().map(Uuid::to_string).collect();
        let unit_ids: Vec<String> = user_unit_ids.iter().map(Uuid::to_string).collect();
        let roles: Vec<String> = user_roles.to_vec();

        sqlx::query_as::<_, DocumentSummary>(
            r#"
            SELECT
                id, title, category::text AS category, file_name, mime_type, size_bytes, folder_id, created_at
            FROM documents
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND (
                -- Creator always has access
                created_by = $2
                -- Organization-wide access
                OR access_scope = 'organization'
                -- Building-based access
                OR (access_scope = 'building' AND access_target_ids ?| $3::text[])
                -- Unit-based access
                OR (access_scope = 'unit' AND access_target_ids ?| $4::text[])
                -- Role-based access
                OR (access_scope = 'role' AND access_roles ?| $5::text[])
                -- Specific user access
                OR (access_scope = 'users' AND access_target_ids ? $2::text)
              )
              AND ($6::uuid IS NULL OR folder_id = $6)
              AND ($7::text IS NULL OR category = $7::document_category)
              AND ($8::text IS NULL OR title ILIKE '%' || $8 || '%')
            ORDER BY created_at DESC
            LIMIT $9 OFFSET $10
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&building_ids)
        .bind(&unit_ids)
        .bind(&roles)
        .bind(query.folder_id)
        .bind(&query.category)
        .bind(&query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Count documents accessible by a specific user with RLS context (GH #1413).
    ///
    /// Counterpart to [`Self::list_accessible_rls`]; the `WHERE` predicate is
    /// kept identical so the non-manager list and its total never disagree.
    #[allow(clippy::too_many_arguments)]
    pub async fn count_accessible_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        user_building_ids: &[Uuid],
        user_unit_ids: &[Uuid],
        user_roles: &[String],
        query: DocumentListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // `?|` is `jsonb ?| text[]` — right operand must be text[], not jsonb.
        let building_ids: Vec<String> = user_building_ids.iter().map(Uuid::to_string).collect();
        let unit_ids: Vec<String> = user_unit_ids.iter().map(Uuid::to_string).collect();
        let roles: Vec<String> = user_roles.to_vec();

        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM documents
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND (
                created_by = $2
                OR access_scope = 'organization'
                OR (access_scope = 'building' AND access_target_ids ?| $3::text[])
                OR (access_scope = 'unit' AND access_target_ids ?| $4::text[])
                OR (access_scope = 'role' AND access_roles ?| $5::text[])
                OR (access_scope = 'users' AND access_target_ids ? $2::text)
              )
              AND ($6::uuid IS NULL OR folder_id = $6)
              AND ($7::text IS NULL OR category = $7::document_category)
              AND ($8::text IS NULL OR title ILIKE '%' || $8 || '%')
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&building_ids)
        .bind(&unit_ids)
        .bind(&roles)
        .bind(query.folder_id)
        .bind(&query.category)
        .bind(&query.search)
        .fetch_one(executor)
        .await?;

        Ok(row.get("count"))
    }

    /// List documents accessible by user (simplified) with RLS context.
    /// Used when building/unit context is not available.
    pub async fn list_accessible_simple_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        user_role: &str,
        query: DocumentListQuery,
    ) -> Result<Vec<DocumentSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, DocumentSummary>(
            r#"
            SELECT
                id, title, category::text AS category, file_name, mime_type, size_bytes, folder_id, created_at
            FROM documents
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND (
                -- Creator always has access
                created_by = $2
                -- Organization-wide access
                OR access_scope = 'organization'
                -- Role-based access (check if user's role is in the access_roles array)
                OR (access_scope = 'role' AND access_roles ? $3)
              )
              AND ($4::uuid IS NULL OR folder_id = $4)
              AND ($5::text IS NULL OR category = $5::document_category)
              AND ($6::text IS NULL OR title ILIKE '%' || $6 || '%')
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(user_role)
        .bind(query.folder_id)
        .bind(&query.category)
        .bind(&query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Count documents accessible by user (simplified) with RLS context.
    pub async fn count_accessible_simple_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        user_role: &str,
        query: DocumentListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM documents
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND (
                created_by = $2
                OR access_scope = 'organization'
                OR (access_scope = 'role' AND access_roles ? $3)
              )
              AND ($4::uuid IS NULL OR folder_id = $4)
              AND ($5::text IS NULL OR category = $5::document_category)
              AND ($6::text IS NULL OR title ILIKE '%' || $6 || '%')
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(user_role)
        .bind(query.folder_id)
        .bind(&query.category)
        .bind(&query.search)
        .fetch_one(executor)
        .await?;

        Ok(row.get("count"))
    }

    /// Update a document with RLS context.
    pub async fn update_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateDocument,
    ) -> Result<Document, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let access_target_ids = data
            .access_target_ids
            .map(|v| serde_json::to_value(v).unwrap());
        let access_roles = data.access_roles.map(|v| serde_json::to_value(v).unwrap());

        sqlx::query_as::<_, Document>(
            r#"
            UPDATE documents
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                category = COALESCE($4::document_category, category),
                folder_id = COALESCE($5, folder_id),
                access_scope = COALESCE($6::document_access_scope, access_scope),
                access_target_ids = COALESCE($7, access_target_ids),
                access_roles = COALESCE($8, access_roles),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING
                id, organization_id, folder_id, title, description,
                category::text AS category, file_key, file_name, mime_type,
                size_bytes, access_scope::text AS access_scope,
                access_target_ids, access_roles, created_by, created_at,
                updated_at, deleted_at, version_number, parent_document_id,
                is_current_version, template_id, generation_metadata
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.category)
        .bind(data.folder_id)
        .bind(&data.access_scope)
        .bind(&access_target_ids)
        .bind(&access_roles)
        .fetch_one(executor)
        .await
    }

    /// Move a document to a folder with RLS context.
    pub async fn move_document_rls<'e, E>(
        &self,
        executor: E,
        data: MoveDocument,
    ) -> Result<Document, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // NOTE: `RETURNING *` returns `category` / `access_scope` as their native
        // Postgres ENUM types (`document_category` / `document_access_scope`),
        // which SQLx cannot decode into the `String` fields on `Document` — that
        // produced a 500 on every move (the column list mirrors `find_by_id_rls`,
        // casting the enums to text).
        sqlx::query_as::<_, Document>(
            r#"
            UPDATE documents
            SET folder_id = $2, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING
                id, organization_id, folder_id, title, description,
                category::text AS category, file_key, file_name, mime_type,
                size_bytes, access_scope::text AS access_scope,
                access_target_ids, access_roles, created_by, created_at,
                updated_at, deleted_at, version_number, parent_document_id,
                is_current_version, template_id, generation_metadata
            "#,
        )
        .bind(data.document_id)
        .bind(data.folder_id)
        .fetch_one(executor)
        .await
    }

    /// Delete a document (soft delete) with RLS context.
    pub async fn delete_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE documents
            SET deleted_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Check if user has access to a document with RLS context (Story 7A.3).
    pub async fn check_access_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        user_id: Uuid,
        user_building_ids: &[Uuid],
        user_unit_ids: &[Uuid],
        user_roles: &[String],
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // `?|` is `jsonb ?| text[]` — right operand must be text[], not jsonb.
        let building_ids: Vec<String> = user_building_ids.iter().map(Uuid::to_string).collect();
        let unit_ids: Vec<String> = user_unit_ids.iter().map(Uuid::to_string).collect();
        let roles: Vec<String> = user_roles.to_vec();

        let row = sqlx::query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM documents
                WHERE id = $1
                  AND deleted_at IS NULL
                  AND (
                    created_by = $2
                    OR access_scope = 'organization'
                    OR (access_scope = 'building' AND access_target_ids ?| $3::text[])
                    OR (access_scope = 'unit' AND access_target_ids ?| $4::text[])
                    OR (access_scope = 'role' AND access_roles ?| $5::text[])
                    OR (access_scope = 'users' AND access_target_ids ? $2::text)
                  )
            ) as has_access
            "#,
        )
        .bind(document_id)
        .bind(user_id)
        .bind(&building_ids)
        .bind(&unit_ids)
        .bind(&roles)
        .fetch_one(executor)
        .await?;

        Ok(row.get("has_access"))
    }

    /// Resolve the caller's `building` and `unit` access-scope memberships.
    ///
    /// A user is a member of a unit (and, transitively, its building) when they
    /// are an **active unit owner** (`unit_owners.status = 'active'` and not
    /// expired) or a **current unit resident** (`unit_residents.end_date IS
    /// NULL`). Returns `(building_ids, unit_ids)` — the inputs the in-memory
    /// download/preview gate and `check_access_rls` need to resolve
    /// `building`/`unit`-scoped documents.
    ///
    /// RLS-scoped: only memberships whose building belongs to the connection's
    /// org are visible, mirroring `BuildingRepository::can_user_access_building`.
    pub async fn user_scope_memberships_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<(Vec<Uuid>, Vec<Uuid>), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT u.id AS unit_id, u.building_id AS building_id
            FROM units u
            WHERE EXISTS (
                SELECT 1 FROM unit_owners uo
                WHERE uo.unit_id = u.id
                  AND uo.user_id = $1
                  AND uo.status = 'active'
                  AND uo.valid_until IS NULL
            )
            OR EXISTS (
                SELECT 1 FROM unit_residents ur
                WHERE ur.unit_id = u.id
                  AND ur.user_id = $1
                  AND ur.end_date IS NULL
            )
            "#,
        )
        .bind(user_id)
        .fetch_all(executor)
        .await?;

        let mut unit_ids = Vec::with_capacity(rows.len());
        let mut building_ids = Vec::new();
        for row in &rows {
            unit_ids.push(row.get::<Uuid, _>("unit_id"));
            let building_id: Uuid = row.get("building_id");
            if !building_ids.contains(&building_id) {
                building_ids.push(building_id);
            }
        }
        Ok((building_ids, unit_ids))
    }

    // ========================================================================
    // Legacy Document Operations (Story 7A.1) - migrate to RLS versions
    // ========================================================================

    /// Create a new document.
    ///
    /// **Deprecated**: Use `create_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use create_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn create(&self, data: CreateDocument) -> Result<Document, SqlxError> {
        self.create_rls(&self.pool, data).await
    }

    /// Find document by ID.
    ///
    /// **Deprecated**: Use `find_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_by_id_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Document>, SqlxError> {
        self.find_by_id_rls(&self.pool, id).await
    }

    /// Find a document by ID for the public share-token path.
    ///
    /// The public share endpoints (`/api/v1/documents/shared/{token}`) are
    /// unauthenticated: the share token itself is the authorization grant, and
    /// no `app.current_org_id` is available. Under `FORCE ROW LEVEL SECURITY`
    /// on `documents` (#754) a plain pool query would be filtered to zero rows,
    /// so this helper acquires a dedicated connection, sets super-admin RLS
    /// context for the single lookup, then clears it before returning the
    /// connection to the pool. Callers MUST have already validated the share
    /// token (and any password) before calling this.
    pub async fn find_by_id_for_share(&self, id: Uuid) -> Result<Option<Document>, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        // Authorize via the validated share token, not org context.
        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = self.find_by_id_rls(&mut *conn, id).await;

        // Always clear context before the connection returns to the pool to
        // prevent super-admin context bleeding into a later request.
        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::error!(
                error = %e,
                "SECURITY: failed to clear RLS context after share-document lookup"
            );
        }

        result
    }

    /// Find a share by token for the public share-token path.
    ///
    /// The public share endpoints (`/api/v1/documents/shared/{token}`) are
    /// unauthenticated: the share token itself is the authorization grant, and
    /// no `app.current_org_id` is available. `document_shares` is under
    /// `FORCE ROW LEVEL SECURITY` (#754 / PAP-21), so a raw-pool query carrying
    /// stale (or no) org context would be filtered to zero rows. This helper
    /// acquires a dedicated connection, sets super-admin RLS context for the
    /// single lookup, then clears it before returning the connection to the
    /// pool. Expiry / revocation are still enforced by the SQL predicate in
    /// `find_share_by_token_rls`.
    pub async fn find_share_by_token_for_share(
        &self,
        token: &str,
    ) -> Result<Option<DocumentShare>, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        // Authorize via the validated share token, not org context.
        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = self.find_share_by_token_rls(&mut *conn, token).await;

        // Always clear context before the connection returns to the pool to
        // prevent super-admin context bleeding into a later request.
        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::error!(
                error = %e,
                "SECURITY: failed to clear RLS context after share-token lookup"
            );
        }

        result
    }

    /// Verify a share's password for the public share-token path.
    ///
    /// Like `find_share_by_token_for_share`, the share row lives under FORCE
    /// RLS, so the lookup needed to read `password_hash` runs on a dedicated
    /// connection with super-admin context that is always cleared afterwards.
    /// Callers MUST have already validated the share token before calling this.
    pub async fn verify_share_password_for_share(
        &self,
        share_id: Uuid,
        password: &str,
    ) -> Result<bool, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let share = self.find_share_by_id_rls(&mut *conn, share_id).await;

        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::error!(
                error = %e,
                "SECURITY: failed to clear RLS context after share-password lookup"
            );
        }

        match share? {
            Some(s) => match s.password_hash {
                Some(hash) => Ok(verify_password(password, &hash)),
                None => Ok(true), // No password required
            },
            None => Ok(false),
        }
    }

    /// Log a public share access for the public share-token path.
    ///
    /// `document_share_access_log` is under FORCE RLS (#754 / PAP-21); the
    /// insert is authorized by the already-validated share token, so it runs on
    /// a dedicated connection with super-admin context that is always cleared
    /// afterwards (mirroring `find_by_id_for_share`).
    pub async fn log_share_access_for_share(
        &self,
        data: LogShareAccess,
    ) -> Result<ShareAccessLog, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = self.log_share_access_rls(&mut *conn, data).await;

        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::error!(
                error = %e,
                "SECURITY: failed to clear RLS context after share-access log insert"
            );
        }

        result
    }

    /// Find document by ID with details.
    ///
    /// **Deprecated**: Use `find_by_id_with_details_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_by_id_with_details_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn find_by_id_with_details(
        &self,
        id: Uuid,
    ) -> Result<Option<DocumentWithDetails>, SqlxError> {
        self.find_by_id_with_details_rls(&self.pool, id).await
    }

    /// List documents for an organization with filters.
    ///
    /// **Deprecated**: Use `list_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use list_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn list(
        &self,
        org_id: Uuid,
        query: DocumentListQuery,
    ) -> Result<Vec<DocumentSummary>, SqlxError> {
        self.list_rls(&self.pool, org_id, query).await
    }

    /// Count documents matching query.
    ///
    /// **Deprecated**: Use `count_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use count_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn count(&self, org_id: Uuid, query: DocumentListQuery) -> Result<i64, SqlxError> {
        self.count_rls(&self.pool, org_id, query).await
    }

    /// List documents accessible by a specific user (Story 7A.3).
    ///
    /// **Deprecated**: Use `list_accessible_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_accessible_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn list_accessible(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        user_building_ids: &[Uuid],
        user_unit_ids: &[Uuid],
        user_roles: &[String],
        query: DocumentListQuery,
    ) -> Result<Vec<DocumentSummary>, SqlxError> {
        self.list_accessible_rls(
            &self.pool,
            org_id,
            user_id,
            user_building_ids,
            user_unit_ids,
            user_roles,
            query,
        )
        .await
    }

    /// List documents accessible by user (simplified - org-wide + own documents + role-based).
    /// Used when building/unit context is not available.
    ///
    /// **Deprecated**: Use `list_accessible_simple_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_accessible_simple_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn list_accessible_simple(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        user_role: &str,
        query: DocumentListQuery,
    ) -> Result<Vec<DocumentSummary>, SqlxError> {
        self.list_accessible_simple_rls(&self.pool, org_id, user_id, user_role, query)
            .await
    }

    /// Count documents accessible by user (simplified).
    ///
    /// **Deprecated**: Use `count_accessible_simple_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_accessible_simple_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn count_accessible_simple(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        user_role: &str,
        query: DocumentListQuery,
    ) -> Result<i64, SqlxError> {
        self.count_accessible_simple_rls(&self.pool, org_id, user_id, user_role, query)
            .await
    }

    /// Update a document.
    ///
    /// **Deprecated**: Use `update_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use update_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn update(&self, id: Uuid, data: UpdateDocument) -> Result<Document, SqlxError> {
        self.update_rls(&self.pool, id, data).await
    }

    /// Move a document to a folder.
    ///
    /// **Deprecated**: Use `move_document_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use move_document_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn move_document(&self, data: MoveDocument) -> Result<Document, SqlxError> {
        self.move_document_rls(&self.pool, data).await
    }

    /// Delete a document (soft delete).
    ///
    /// **Deprecated**: Use `delete_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use delete_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn delete(&self, id: Uuid) -> Result<(), SqlxError> {
        self.delete_rls(&self.pool, id).await
    }

    /// Check if user has access to a document (Story 7A.3).
    ///
    /// **Deprecated**: Use `check_access_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use check_access_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn check_access(
        &self,
        document_id: Uuid,
        user_id: Uuid,
        user_building_ids: &[Uuid],
        user_unit_ids: &[Uuid],
        user_roles: &[String],
    ) -> Result<bool, SqlxError> {
        self.check_access_rls(
            &self.pool,
            document_id,
            user_id,
            user_building_ids,
            user_unit_ids,
            user_roles,
        )
        .await
    }
}

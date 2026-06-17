//! Document repository (Epic 7A: Basic Document Management, Epic 7B: Document Versioning, Epic 28: Document Intelligence).
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
//! async fn create_document(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateDocument>,
//! ) -> Result<Json<Document>> {
//!     let document = state.document_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(document))
//! }
//! ```

use crate::models::{
    access_scope, ClassificationFeedback, CreateDocument, CreateDocumentVersion, CreateFolder,
    CreateShare, Document, DocumentClassificationHistory, DocumentFolder,
    DocumentIntelligenceStats, DocumentListQuery, DocumentOcrQueue, DocumentSearchRequest,
    DocumentSearchResponse, DocumentSearchResult, DocumentShare, DocumentSummarizationQueue,
    DocumentSummary, DocumentVersion, DocumentVersionHistory, DocumentWithDetails, FolderTreeNode,
    FolderWithCount, GenerateSummaryRequest, LogShareAccess, MoveDocument, ShareAccessLog,
    ShareWithDocument, UpdateDocument, UpdateFolder,
};
use chrono::{NaiveDate, Utc};
use sqlx::{Error as SqlxError, Executor, PgPool, Postgres, Row};
use uuid::Uuid;

/// Repository for document operations.
#[derive(Debug, Clone)]
pub struct DocumentRepository {
    pool: PgPool,
}

impl DocumentRepository {
    /// Create a new document repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware Folder Operations (Story 7A.2)
    // ========================================================================

    /// Create a new folder with RLS context.
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_folder_rls<'e, E>(
        &self,
        executor: E,
        data: CreateFolder,
    ) -> Result<DocumentFolder, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentFolder>(
            r#"
            INSERT INTO document_folders (organization_id, parent_id, name, description, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.parent_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Find folder by ID, scoped to `org_id` (RLS context).
    ///
    /// The explicit `organization_id = $2` predicate is defense-in-depth on top
    /// of the `folder_tenant_isolation` RLS policy: it guarantees cross-org rows
    /// are invisible (lookup → `None` → 404) even when the connection is a
    /// Postgres SUPERUSER, which bypasses RLS entirely (FORCE ROW LEVEL SECURITY
    /// binds only the table OWNER, not superusers). The default test/CI
    /// `cargo test` job connects as the `postgres` superuser, so without this
    /// predicate the cross-org IDOR probes (#679) leak Org A's folder to an
    /// Org B caller. Mirrors the explicit-org-scoping pattern used by the other
    /// `*_cross_org_idor` suites.
    pub async fn find_folder_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<DocumentFolder>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentFolder>(
            r#"
            SELECT * FROM document_folders
            WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get all folders for an organization with RLS context.
    pub async fn get_folders_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<FolderWithCount>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query(
            r#"
            SELECT
                f.id, f.organization_id, f.parent_id, f.name, f.description,
                f.created_by, f.created_at, f.updated_at, f.deleted_at,
                COALESCE(doc_count.count, 0)::bigint as document_count,
                COALESCE(sub_count.count, 0)::bigint as subfolder_count
            FROM document_folders f
            LEFT JOIN (
                SELECT folder_id, COUNT(*) as count
                FROM documents
                WHERE deleted_at IS NULL
                GROUP BY folder_id
            ) doc_count ON doc_count.folder_id = f.id
            LEFT JOIN (
                SELECT parent_id, COUNT(*) as count
                FROM document_folders
                WHERE deleted_at IS NULL
                GROUP BY parent_id
            ) sub_count ON sub_count.parent_id = f.id
            WHERE f.organization_id = $1
              AND f.deleted_at IS NULL
              AND (f.parent_id = $2 OR ($2 IS NULL AND f.parent_id IS NULL))
            ORDER BY f.name
            "#,
        )
        .bind(org_id)
        .bind(parent_id)
        .fetch_all(executor)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| FolderWithCount {
                folder: DocumentFolder {
                    id: r.get("id"),
                    organization_id: r.get("organization_id"),
                    parent_id: r.get("parent_id"),
                    name: r.get("name"),
                    description: r.get("description"),
                    created_by: r.get("created_by"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                    deleted_at: r.get("deleted_at"),
                },
                document_count: r.get("document_count"),
                subfolder_count: r.get("subfolder_count"),
            })
            .collect())
    }

    /// Get folder tree for an organization with RLS context.
    pub async fn get_folder_tree_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<FolderTreeNode>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query(
            r#"
            WITH folder_counts AS (
                SELECT folder_id, COUNT(*) as doc_count
                FROM documents
                WHERE organization_id = $1 AND deleted_at IS NULL
                GROUP BY folder_id
            )
            SELECT
                f.id,
                f.name,
                f.parent_id,
                COALESCE(fc.doc_count, 0) as document_count
            FROM document_folders f
            LEFT JOIN folder_counts fc ON fc.folder_id = f.id
            WHERE f.organization_id = $1 AND f.deleted_at IS NULL
            ORDER BY f.parent_id NULLS FIRST, f.name
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await?;

        // Build tree structure
        let nodes: Vec<FolderTreeNode> = rows
            .iter()
            .map(|row| FolderTreeNode {
                id: row.get("id"),
                name: row.get("name"),
                parent_id: row.get("parent_id"),
                document_count: row.get("document_count"),
                children: None,
            })
            .collect();

        Ok(build_folder_tree(nodes))
    }

    /// Update a folder with RLS context.
    pub async fn update_folder_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateFolder,
    ) -> Result<DocumentFolder, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentFolder>(
            r#"
            UPDATE document_folders
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                parent_id = COALESCE($4, parent_id),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.parent_id)
        .fetch_one(executor)
        .await
    }

    /// Check if a folder is a descendant of another folder with RLS context.
    /// Used to prevent circular references when updating parent_id.
    pub async fn is_descendant_of_rls<'e, E>(
        &self,
        executor: E,
        folder_id: Uuid,
        potential_ancestor_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            WITH RECURSIVE ancestors AS (
                SELECT id, parent_id FROM document_folders WHERE id = $1
                UNION ALL
                SELECT f.id, f.parent_id FROM document_folders f
                JOIN ancestors a ON f.id = a.parent_id
                WHERE f.deleted_at IS NULL
            )
            SELECT EXISTS(SELECT 1 FROM ancestors WHERE id = $2) as is_descendant
            "#,
        )
        .bind(folder_id)
        .bind(potential_ancestor_id)
        .fetch_one(executor)
        .await?;

        Ok(row.get("is_descendant"))
    }

    /// Delete a folder (soft delete) with RLS context.
    pub async fn delete_folder_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        _cascade: bool,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Note: For RLS version, we only support non-cascade delete to avoid
        // needing multiple executor calls. Use the legacy version for cascade.
        // Move documents to root and delete the folder.
        sqlx::query(
            r#"
            WITH moved_docs AS (
                UPDATE documents
                SET folder_id = NULL
                WHERE folder_id = $1 AND deleted_at IS NULL
            )
            UPDATE document_folders
            SET deleted_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Count documents in a folder with RLS context.
    pub async fn count_documents_in_folder_rls<'e, E>(
        &self,
        executor: E,
        folder_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM documents
            WHERE folder_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(folder_id)
        .fetch_one(executor)
        .await?;

        Ok(row.get("count"))
    }

    // ========================================================================
    // Legacy Folder Operations (Story 7A.2) - migrate to RLS versions
    // ========================================================================

    /// Create a new folder.
    ///
    /// **Deprecated**: Use `create_folder_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_folder_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn create_folder(&self, data: CreateFolder) -> Result<DocumentFolder, SqlxError> {
        self.create_folder_rls(&self.pool, data).await
    }

    /// Get all folders for an organization.
    ///
    /// **Deprecated**: Use `get_folders_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_folders_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_folders(
        &self,
        org_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<FolderWithCount>, SqlxError> {
        self.get_folders_rls(&self.pool, org_id, parent_id).await
    }

    /// Get folder tree for an organization.
    ///
    /// **Deprecated**: Use `get_folder_tree_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_folder_tree_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_folder_tree(&self, org_id: Uuid) -> Result<Vec<FolderTreeNode>, SqlxError> {
        self.get_folder_tree_rls(&self.pool, org_id).await
    }

    /// Update a folder.
    ///
    /// **Deprecated**: Use `update_folder_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use update_folder_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn update_folder(
        &self,
        id: Uuid,
        data: UpdateFolder,
    ) -> Result<DocumentFolder, SqlxError> {
        self.update_folder_rls(&self.pool, id, data).await
    }

    /// Check if a folder is a descendant of another folder.
    /// Used to prevent circular references when updating parent_id.
    ///
    /// **Deprecated**: Use `is_descendant_of_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use is_descendant_of_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn is_descendant_of(
        &self,
        folder_id: Uuid,
        potential_ancestor_id: Uuid,
    ) -> Result<bool, SqlxError> {
        self.is_descendant_of_rls(&self.pool, folder_id, potential_ancestor_id)
            .await
    }

    /// Delete a folder (soft delete).
    ///
    /// **Deprecated**: Use `delete_folder_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use delete_folder_rls with RlsConnection instead"
    )]
    pub async fn delete_folder(&self, id: Uuid, cascade: bool) -> Result<(), SqlxError> {
        if cascade {
            // Delete all documents in folder
            sqlx::query(
                r#"
                UPDATE documents
                SET deleted_at = NOW()
                WHERE folder_id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;

            // Delete all subfolders recursively
            sqlx::query(
                r#"
                WITH RECURSIVE subfolders AS (
                    SELECT id FROM document_folders WHERE id = $1
                    UNION ALL
                    SELECT f.id FROM document_folders f
                    JOIN subfolders s ON f.parent_id = s.id
                )
                UPDATE document_folders
                SET deleted_at = NOW()
                WHERE id IN (SELECT id FROM subfolders)
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            // Just delete the folder, move documents to root
            sqlx::query(
                r#"
                UPDATE documents
                SET folder_id = NULL
                WHERE folder_id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                UPDATE document_folders
                SET deleted_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Count documents in a folder.
    ///
    /// **Deprecated**: Use `count_documents_in_folder_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_documents_in_folder_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn count_documents_in_folder(&self, folder_id: Uuid) -> Result<i64, SqlxError> {
        self.count_documents_in_folder_rls(&self.pool, folder_id)
            .await
    }

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
                u.full_name as created_by_name,
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
                category: r.get("category"),
                file_key: r.get("file_key"),
                file_name: r.get("file_name"),
                mime_type: r.get("mime_type"),
                size_bytes: r.get("size_bytes"),
                access_scope: r.get("access_scope"),
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
                category = COALESCE($4, category),
                folder_id = COALESCE($5, folder_id),
                access_scope = COALESCE($6, access_scope),
                access_target_ids = COALESCE($7, access_target_ids),
                access_roles = COALESCE($8, access_roles),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
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

    // ========================================================================
    // RLS-aware Share Operations (Story 7A.5)
    // ========================================================================

    /// Create a new share with RLS context.
    pub async fn create_share_rls<'e, E>(
        &self,
        executor: E,
        data: CreateShare,
    ) -> Result<DocumentShare, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let share_token = if data.share_type == crate::models::share_type::LINK {
            Some(generate_share_token())
        } else {
            None
        };

        let password_hash = match data.password.as_ref() {
            Some(password) => Some(hash_password(password)?),
            None => None,
        };

        sqlx::query_as::<_, DocumentShare>(
            r#"
            INSERT INTO document_shares (
                document_id, share_type, target_id, target_role, shared_by,
                share_token, password_hash, expires_at
            )
            VALUES ($1, $2::document_share_type, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, document_id, share_type::text AS share_type, target_id,
                target_role, shared_by, share_token, password_hash, expires_at,
                revoked_at, created_at
            "#,
        )
        .bind(data.document_id)
        .bind(&data.share_type)
        .bind(data.target_id)
        .bind(&data.target_role)
        .bind(data.shared_by)
        .bind(&share_token)
        .bind(&password_hash)
        .bind(data.expires_at)
        .fetch_one(executor)
        .await
    }

    /// Find share by ID with RLS context.
    pub async fn find_share_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<DocumentShare>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentShare>(
            r#"
            SELECT
                id, document_id, share_type::text AS share_type, target_id,
                target_role, shared_by, share_token, password_hash, expires_at,
                revoked_at, created_at
            FROM document_shares
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Find share by token with RLS context.
    pub async fn find_share_by_token_rls<'e, E>(
        &self,
        executor: E,
        token: &str,
    ) -> Result<Option<DocumentShare>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentShare>(
            r#"
            SELECT
                id, document_id, share_type::text AS share_type, target_id,
                target_role, shared_by, share_token, password_hash, expires_at,
                revoked_at, created_at
            FROM document_shares
            WHERE share_token = $1
              AND revoked_at IS NULL
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(token)
        .fetch_optional(executor)
        .await
    }

    /// Get shares for a document with RLS context.
    pub async fn get_shares_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<Vec<ShareWithDocument>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query(
            r#"
            SELECT
                s.*,
                d.title as document_title,
                u.full_name as shared_by_name
            FROM document_shares s
            JOIN documents d ON d.id = s.document_id
            JOIN users u ON u.id = s.shared_by
            WHERE s.document_id = $1 AND s.revoked_at IS NULL
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(executor)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ShareWithDocument {
                share: DocumentShare {
                    id: r.get("id"),
                    document_id: r.get("document_id"),
                    share_type: r.get("share_type"),
                    target_id: r.get("target_id"),
                    target_role: r.get("target_role"),
                    shared_by: r.get("shared_by"),
                    share_token: r.get("share_token"),
                    password_hash: r.get("password_hash"),
                    expires_at: r.get("expires_at"),
                    revoked_at: r.get("revoked_at"),
                    created_at: r.get("created_at"),
                },
                document_title: r.get("document_title"),
                shared_by_name: r.get("shared_by_name"),
            })
            .collect())
    }

    /// Revoke a share with RLS context.
    pub async fn revoke_share_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE document_shares
            SET revoked_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Log share access with RLS context.
    pub async fn log_share_access_rls<'e, E>(
        &self,
        executor: E,
        data: LogShareAccess,
    ) -> Result<ShareAccessLog, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ShareAccessLog>(
            r#"
            INSERT INTO document_share_access_log (share_id, accessed_by, ip_address)
            VALUES ($1, $2, $3::inet)
            RETURNING *
            "#,
        )
        .bind(data.share_id)
        .bind(data.accessed_by)
        .bind(&data.ip_address)
        .fetch_one(executor)
        .await
    }

    /// Get share access log with RLS context.
    pub async fn get_share_access_log_rls<'e, E>(
        &self,
        executor: E,
        share_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<ShareAccessLog>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = limit.unwrap_or(100).min(500);

        sqlx::query_as::<_, ShareAccessLog>(
            r#"
            SELECT * FROM document_share_access_log
            WHERE share_id = $1
            ORDER BY accessed_at DESC
            LIMIT $2
            "#,
        )
        .bind(share_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // Legacy Share Operations (Story 7A.5) - migrate to RLS versions
    // ========================================================================

    /// Create a new share.
    ///
    /// **Deprecated**: Use `create_share_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_share_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn create_share(&self, data: CreateShare) -> Result<DocumentShare, SqlxError> {
        self.create_share_rls(&self.pool, data).await
    }

    /// Find share by ID.
    ///
    /// **Deprecated**: Use `find_share_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_share_by_id_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn find_share_by_id(&self, id: Uuid) -> Result<Option<DocumentShare>, SqlxError> {
        self.find_share_by_id_rls(&self.pool, id).await
    }

    /// Find share by token.
    ///
    /// **Deprecated**: Use `find_share_by_token_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_share_by_token_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn find_share_by_token(
        &self,
        token: &str,
    ) -> Result<Option<DocumentShare>, SqlxError> {
        self.find_share_by_token_rls(&self.pool, token).await
    }

    /// Get shares for a document.
    ///
    /// **Deprecated**: Use `get_shares_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_shares_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_shares(&self, document_id: Uuid) -> Result<Vec<ShareWithDocument>, SqlxError> {
        self.get_shares_rls(&self.pool, document_id).await
    }

    /// Revoke a share.
    ///
    /// **Deprecated**: Use `revoke_share_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use revoke_share_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn revoke_share(&self, id: Uuid) -> Result<(), SqlxError> {
        self.revoke_share_rls(&self.pool, id).await
    }

    /// Verify share password.
    ///
    /// Note: This method requires fetching the share first, so it uses internal deprecated methods.
    #[allow(deprecated)]
    pub async fn verify_share_password(
        &self,
        share_id: Uuid,
        password: &str,
    ) -> Result<bool, SqlxError> {
        let share = self.find_share_by_id(share_id).await?;
        match share {
            Some(s) => {
                if let Some(hash) = s.password_hash {
                    Ok(verify_password(password, &hash))
                } else {
                    Ok(true) // No password required
                }
            }
            None => Ok(false),
        }
    }

    /// Log share access.
    ///
    /// **Deprecated**: Use `log_share_access_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use log_share_access_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn log_share_access(
        &self,
        data: LogShareAccess,
    ) -> Result<ShareAccessLog, SqlxError> {
        self.log_share_access_rls(&self.pool, data).await
    }

    /// Get share access log.
    ///
    /// **Deprecated**: Use `get_share_access_log_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_share_access_log_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_share_access_log(
        &self,
        share_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<ShareAccessLog>, SqlxError> {
        self.get_share_access_log_rls(&self.pool, share_id, limit)
            .await
    }

    // ========================================================================
    // RLS-aware Version Operations (Story 7B.1)
    // ========================================================================

    /// Create a new version of an existing document with RLS context.
    ///
    /// This creates a new document record with:
    /// - An incremented version number
    /// - Reference to the original document (parent_document_id)
    /// - is_current_version set to true (previous versions are auto-updated to false via trigger)
    pub async fn create_version_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        data: CreateDocumentVersion,
    ) -> Result<Document, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Note: This RLS version performs a single query that combines finding the original
        // and creating the new version. For full functionality with multiple queries,
        // use the legacy version or handle the transaction externally.
        sqlx::query_as::<_, Document>(
            r#"
            WITH original AS (
                SELECT * FROM documents WHERE id = $1 AND deleted_at IS NULL
            ),
            next_ver AS (
                SELECT get_next_document_version($1) as version_number
            )
            INSERT INTO documents (
                organization_id, folder_id, title, description, category,
                file_key, file_name, mime_type, size_bytes,
                access_scope, access_target_ids, access_roles, created_by,
                version_number, parent_document_id, is_current_version
            )
            SELECT
                o.organization_id, o.folder_id, o.title, o.description, o.category,
                $2, $3, $4, $5,
                o.access_scope, o.access_target_ids, o.access_roles, $6,
                n.version_number, COALESCE(o.parent_document_id, o.id), true
            FROM original o, next_ver n
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(&data.file_key)
        .bind(&data.file_name)
        .bind(&data.mime_type)
        .bind(data.size_bytes)
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Get version history for a document with RLS context.
    ///
    /// Returns all versions in the chain, ordered by version number (descending).
    pub async fn get_version_history_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<DocumentVersionHistory, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Single query to get document info and all versions
        let rows = sqlx::query(
            r#"
            WITH doc AS (
                SELECT
                    COALESCE(parent_document_id, id) as root_id,
                    title
                FROM documents
                WHERE id = $1 AND deleted_at IS NULL
            )
            SELECT
                d.id,
                d.version_number,
                d.is_current_version,
                d.file_key,
                d.file_name,
                d.mime_type,
                d.size_bytes,
                d.created_by,
                u.name as created_by_name,
                d.created_at,
                doc.root_id,
                doc.title
            FROM documents d
            JOIN users u ON u.id = d.created_by
            CROSS JOIN doc
            WHERE d.deleted_at IS NULL
              AND (d.id = doc.root_id OR d.parent_document_id = doc.root_id)
            ORDER BY d.version_number DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(executor)
        .await?;

        if rows.is_empty() {
            return Err(SqlxError::RowNotFound);
        }

        let root_id: Uuid = rows[0].get("root_id");
        let title: String = rows[0].get("title");

        let versions: Vec<DocumentVersion> = rows
            .into_iter()
            .map(|row| DocumentVersion {
                id: row.get("id"),
                version_number: row.get("version_number"),
                is_current_version: row.get("is_current_version"),
                file_key: row.get("file_key"),
                file_name: row.get("file_name"),
                mime_type: row.get("mime_type"),
                size_bytes: row.get("size_bytes"),
                created_by: row.get("created_by"),
                created_by_name: row.get("created_by_name"),
                created_at: row.get("created_at"),
            })
            .collect();

        let total_versions = versions.len() as i32;

        Ok(DocumentVersionHistory {
            document_id: root_id,
            title,
            total_versions,
            versions,
        })
    }

    /// Get a specific version of a document with RLS context.
    pub async fn get_version_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        version_id: Uuid,
    ) -> Result<Option<DocumentVersion>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentVersion>(
            r#"
            WITH doc AS (
                SELECT COALESCE(parent_document_id, id) as root_id
                FROM documents
                WHERE id = $1 AND deleted_at IS NULL
            )
            SELECT
                d.id,
                d.version_number,
                d.is_current_version,
                d.file_key,
                d.file_name,
                d.mime_type,
                d.size_bytes,
                d.created_by,
                u.name as created_by_name,
                d.created_at
            FROM documents d
            JOIN users u ON u.id = d.created_by
            JOIN doc ON (d.id = doc.root_id OR d.parent_document_id = doc.root_id)
            WHERE d.id = $2 AND d.deleted_at IS NULL
            "#,
        )
        .bind(document_id)
        .bind(version_id)
        .fetch_optional(executor)
        .await
    }

    /// Get the current (latest) version of a document with RLS context.
    pub async fn get_current_version_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<Option<Document>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Document>(
            r#"
            WITH doc AS (
                SELECT COALESCE(parent_document_id, id) as root_id
                FROM documents
                WHERE id = $1 AND deleted_at IS NULL
            )
            SELECT d.* FROM documents d
            JOIN doc ON (d.id = doc.root_id OR d.parent_document_id = doc.root_id)
            WHERE d.deleted_at IS NULL AND d.is_current_version = true
            "#,
        )
        .bind(document_id)
        .fetch_optional(executor)
        .await
    }

    // ========================================================================
    // Legacy Version Operations (Story 7B.1) - migrate to RLS versions
    // ========================================================================

    /// Create a new version of an existing document.
    ///
    /// This creates a new document record with:
    /// - An incremented version number
    /// - Reference to the original document (parent_document_id)
    /// - is_current_version set to true (previous versions are auto-updated to false via trigger)
    ///
    /// **Deprecated**: Use `create_version_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_version_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn create_version(
        &self,
        document_id: Uuid,
        data: CreateDocumentVersion,
    ) -> Result<Document, SqlxError> {
        // First, get the original document to copy metadata
        let original = self
            .find_by_id(document_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Determine the root document ID (the first version in the chain)
        let root_id = original.root_document_id();

        // Get the next version number using the database function
        let next_version: i32 = sqlx::query_scalar("SELECT get_next_document_version($1)")
            .bind(document_id)
            .fetch_one(&self.pool)
            .await?;

        // Create the new version with copied metadata
        sqlx::query_as::<_, Document>(
            r#"
            INSERT INTO documents (
                organization_id, folder_id, title, description, category,
                file_key, file_name, mime_type, size_bytes,
                access_scope, access_target_ids, access_roles, created_by,
                version_number, parent_document_id, is_current_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, true)
            RETURNING *
            "#,
        )
        .bind(original.organization_id)
        .bind(original.folder_id)
        .bind(&original.title)
        .bind(&original.description)
        .bind(&original.category)
        .bind(&data.file_key)
        .bind(&data.file_name)
        .bind(&data.mime_type)
        .bind(data.size_bytes)
        .bind(&original.access_scope)
        .bind(&original.access_target_ids)
        .bind(&original.access_roles)
        .bind(data.created_by)
        .bind(next_version)
        .bind(root_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get version history for a document.
    ///
    /// Returns all versions in the chain, ordered by version number (descending).
    ///
    /// **Deprecated**: Use `get_version_history_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_version_history_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_version_history(
        &self,
        document_id: Uuid,
    ) -> Result<DocumentVersionHistory, SqlxError> {
        // First get the document to find the root
        let doc = self
            .find_by_id(document_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        let root_id = doc.root_document_id();

        // Get all versions in the chain
        let versions = sqlx::query_as::<_, DocumentVersion>(
            r#"
            SELECT
                d.id,
                d.version_number,
                d.is_current_version,
                d.file_key,
                d.file_name,
                d.mime_type,
                d.size_bytes,
                d.created_by,
                u.name as created_by_name,
                d.created_at
            FROM documents d
            JOIN users u ON u.id = d.created_by
            WHERE d.deleted_at IS NULL
              AND (d.id = $1 OR d.parent_document_id = $1)
            ORDER BY d.version_number DESC
            "#,
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?;

        let total_versions = versions.len() as i32;

        Ok(DocumentVersionHistory {
            document_id: root_id,
            title: doc.title,
            total_versions,
            versions,
        })
    }

    /// Get a specific version of a document.
    ///
    /// **Deprecated**: Use `get_version_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_version_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_version(
        &self,
        document_id: Uuid,
        version_id: Uuid,
    ) -> Result<Option<DocumentVersion>, SqlxError> {
        // First verify the version belongs to the document chain
        let doc = match self.find_by_id(document_id).await? {
            Some(d) => d,
            None => return Ok(None),
        };

        let root_id = doc.root_document_id();

        sqlx::query_as::<_, DocumentVersion>(
            r#"
            SELECT
                d.id,
                d.version_number,
                d.is_current_version,
                d.file_key,
                d.file_name,
                d.mime_type,
                d.size_bytes,
                d.created_by,
                u.name as created_by_name,
                d.created_at
            FROM documents d
            JOIN users u ON u.id = d.created_by
            WHERE d.id = $1
              AND d.deleted_at IS NULL
              AND (d.id = $2 OR d.parent_document_id = $2)
            "#,
        )
        .bind(version_id)
        .bind(root_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Restore a previous version to become the current version.
    ///
    /// This creates a new version entry with the content from the old version,
    /// making it non-destructive (preserving full history).
    ///
    /// **Deprecated**: Use `restore_version_rls` or handle externally with RLS-enabled connection.
    #[deprecated(
        since = "0.2.276",
        note = "Use restore_version with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn restore_version(
        &self,
        document_id: Uuid,
        version_id: Uuid,
        restored_by: Uuid,
    ) -> Result<Document, SqlxError> {
        // Get the version to restore
        let version_to_restore = self
            .find_by_id(version_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Verify it belongs to the same document chain
        let original_doc = self
            .find_by_id(document_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        if version_to_restore.root_document_id() != original_doc.root_document_id() {
            return Err(SqlxError::RowNotFound);
        }

        // Create a new version based on the old version's content
        let create_version_data = CreateDocumentVersion {
            file_key: version_to_restore.file_key,
            file_name: version_to_restore.file_name,
            mime_type: version_to_restore.mime_type,
            size_bytes: version_to_restore.size_bytes,
            created_by: restored_by,
        };

        self.create_version(document_id, create_version_data).await
    }

    /// Restore a previous version to become the current version, using an
    /// RLS-enabled connection.
    ///
    /// This is the RLS-aware counterpart of [`Self::restore_version`]. All
    /// reads/writes run on the supplied connection (which carries the caller's
    /// `app.current_org_id`), so it stays correct under
    /// `FORCE ROW LEVEL SECURITY` on `documents` (issue #754). The connection is
    /// borrowed mutably because the operation spans multiple queries.
    pub async fn restore_version_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        document_id: Uuid,
        version_id: Uuid,
        restored_by: Uuid,
    ) -> Result<Document, SqlxError> {
        // Get the version to restore (org-scoped via RLS).
        let version_to_restore = self
            .find_by_id_rls(&mut *conn, version_id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Verify it belongs to the same document chain (org-scoped via RLS).
        let original_doc = self
            .find_by_id_rls(&mut *conn, document_id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        if version_to_restore.root_document_id() != original_doc.root_document_id() {
            return Err(SqlxError::RowNotFound);
        }

        // Create a new version based on the old version's content.
        let create_version_data = CreateDocumentVersion {
            file_key: version_to_restore.file_key,
            file_name: version_to_restore.file_name,
            mime_type: version_to_restore.mime_type,
            size_bytes: version_to_restore.size_bytes,
            created_by: restored_by,
        };

        self.create_version_rls(&mut *conn, document_id, create_version_data)
            .await
    }

    /// Get the current (latest) version of a document.
    ///
    /// **Deprecated**: Use `get_current_version_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_current_version_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_current_version(
        &self,
        document_id: Uuid,
    ) -> Result<Option<Document>, SqlxError> {
        // Get the document to find the root
        let doc = match self.find_by_id(document_id).await? {
            Some(d) => d,
            None => return Ok(None),
        };

        let root_id = doc.root_document_id();

        sqlx::query_as::<_, Document>(
            r#"
            SELECT * FROM documents
            WHERE deleted_at IS NULL
              AND is_current_version = true
              AND (id = $1 OR parent_document_id = $1)
            "#,
        )
        .bind(root_id)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Document Intelligence (Epic 28)
    // Note: These methods involve complex operations with queues and stats
    // that often require multiple queries. They are kept as legacy methods
    // for now, but can be migrated to RLS versions as needed.
    // ========================================================================

    // ------------------------------------------------------------------------
    // Story 28.1: OCR Text Extraction
    // ------------------------------------------------------------------------

    /// Queue a document for OCR processing.
    pub async fn queue_for_ocr(
        &self,
        document_id: Uuid,
        priority: Option<i32>,
    ) -> Result<Option<Uuid>, SqlxError> {
        let row = sqlx::query(r#"SELECT queue_document_for_ocr($1, $2) as queue_id"#)
            .bind(document_id)
            .bind(priority.unwrap_or(5))
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("queue_id"))
    }

    /// Queue a document for OCR processing on an RLS-enabled connection.
    ///
    /// RLS-aware counterpart of [`Self::queue_for_ocr`]. The underlying
    /// `queue_document_for_ocr` function reads `documents`, which is under
    /// FORCE ROW LEVEL SECURITY (#754), so it must run on the caller's
    /// org-scoped connection.
    pub async fn queue_for_ocr_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        priority: Option<i32>,
    ) -> Result<Option<Uuid>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(r#"SELECT queue_document_for_ocr($1, $2) as queue_id"#)
            .bind(document_id)
            .bind(priority.unwrap_or(5))
            .fetch_one(executor)
            .await?;

        Ok(row.get("queue_id"))
    }

    /// Get pending OCR queue items for processing.
    pub async fn get_pending_ocr_items(
        &self,
        limit: i64,
    ) -> Result<Vec<DocumentOcrQueue>, SqlxError> {
        sqlx::query_as::<_, DocumentOcrQueue>(
            r#"
            SELECT * FROM document_ocr_queue
            WHERE attempts < max_attempts
              AND next_attempt_at <= NOW()
            ORDER BY priority, next_attempt_at
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Update OCR status for a document.
    pub async fn update_ocr_status(
        &self,
        document_id: Uuid,
        status: &str,
        extracted_text: Option<&str>,
        page_count: Option<i32>,
        confidence: Option<f64>,
        error: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE documents SET
                ocr_status = $2::ocr_status,
                extracted_text = COALESCE($3, extracted_text),
                ocr_page_count = COALESCE($4, ocr_page_count),
                ocr_confidence = COALESCE($5, ocr_confidence),
                ocr_error = $6,
                ocr_processed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(status)
        .bind(extracted_text)
        .bind(page_count)
        .bind(confidence)
        .bind(error)
        .execute(&self.pool)
        .await?;

        // Remove from queue on success or final failure
        if status == "completed" || status == "failed" || status == "not_applicable" {
            sqlx::query("DELETE FROM document_ocr_queue WHERE document_id = $1")
                .bind(document_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Record OCR attempt (for retry logic).
    pub async fn record_ocr_attempt(
        &self,
        document_id: Uuid,
        error: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE document_ocr_queue SET
                attempts = attempts + 1,
                last_error = $2,
                next_attempt_at = NOW() + INTERVAL '5 minutes' * power(2, attempts),
                updated_at = NOW()
            WHERE document_id = $1
            "#,
        )
        .bind(document_id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Story 28.2: Full-Text Search
    // ------------------------------------------------------------------------

    /// Full-text search across documents.
    pub async fn full_text_search(
        &self,
        request: DocumentSearchRequest,
    ) -> Result<DocumentSearchResponse, SqlxError> {
        // `documents` is under FORCE ROW LEVEL SECURITY (#754). This legacy
        // entrypoint acquires a connection and sets the request's org context
        // so the search stays correct, then delegates to the RLS variant.
        let mut conn = self.pool.acquire().await?;
        crate::tenant_context::set_request_context(
            &mut *conn,
            Some(request.organization_id),
            None,
            false,
        )
        .await?;
        let result = self.full_text_search_rls(&mut conn, request).await;
        let _ = crate::tenant_context::clear_request_context(&mut *conn).await;
        result
    }

    /// Full-text search across documents on an RLS-enabled connection.
    ///
    /// RLS-aware counterpart of [`Self::full_text_search`]. Runs both the
    /// result and count queries on the supplied org-scoped connection, so it
    /// stays correct under FORCE ROW LEVEL SECURITY on `documents` (#754).
    pub async fn full_text_search_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        request: DocumentSearchRequest,
    ) -> Result<DocumentSearchResponse, SqlxError> {
        let limit = request.limit.unwrap_or(20).min(100);
        let offset = request.offset.unwrap_or(0);

        // Parse search query for PostgreSQL full-text search
        let ts_query = format!(
            "{}:*",
            request
                .query
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(":* & ")
        );

        let rows = sqlx::query(
            r#"
            SELECT
                d.id, d.title, d.category::text AS category, d.file_name, d.mime_type, d.size_bytes,
                d.folder_id, d.created_at,
                ts_rank_cd(d.search_vector, to_tsquery('english', $2)) as rank,
                ts_headline('english', COALESCE(d.extracted_text, d.description, ''),
                    to_tsquery('english', $2),
                    'StartSel=<mark>, StopSel=</mark>, MaxWords=50, MinWords=20') as headline,
                CASE
                    WHEN d.title ILIKE '%' || $6 || '%' THEN 'title'
                    WHEN d.extracted_text ILIKE '%' || $6 || '%' THEN 'content'
                    WHEN d.description ILIKE '%' || $6 || '%' THEN 'description'
                    ELSE 'other'
                END as matched_field
            FROM documents d
            WHERE d.organization_id = $1
              AND d.deleted_at IS NULL
              AND d.search_vector @@ to_tsquery('english', $2)
              AND ($3::uuid IS NULL OR d.folder_id = $3)
              AND ($4::text IS NULL OR d.category = $4::document_category)
            ORDER BY rank DESC, d.created_at DESC
            LIMIT $5 OFFSET $7
            "#,
        )
        .bind(request.organization_id)
        .bind(&ts_query)
        .bind(request.folder_id)
        .bind(&request.category)
        .bind(limit)
        .bind(&request.query)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await?;

        // Get total count
        let count_row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM documents d
            WHERE d.organization_id = $1
              AND d.deleted_at IS NULL
              AND d.search_vector @@ to_tsquery('english', $2)
              AND ($3::uuid IS NULL OR d.folder_id = $3)
              AND ($4::text IS NULL OR d.category = $4::document_category)
            "#,
        )
        .bind(request.organization_id)
        .bind(&ts_query)
        .bind(request.folder_id)
        .bind(&request.category)
        .fetch_one(&mut *conn)
        .await?;

        let total: i64 = count_row.get("count");

        let results: Vec<DocumentSearchResult> = rows
            .into_iter()
            .map(|row| {
                let matched_field: String = row.get("matched_field");
                DocumentSearchResult {
                    document: DocumentSummary {
                        id: row.get("id"),
                        title: row.get("title"),
                        category: row.get("category"),
                        file_name: row.get("file_name"),
                        mime_type: row.get("mime_type"),
                        size_bytes: row.get("size_bytes"),
                        folder_id: row.get("folder_id"),
                        created_at: row.get("created_at"),
                    },
                    rank: row.get("rank"),
                    headline: row.get("headline"),
                    matched_fields: vec![matched_field],
                }
            })
            .collect();

        Ok(DocumentSearchResponse {
            results,
            total,
            query: request.query,
        })
    }

    // ------------------------------------------------------------------------
    // Story 28.3: Auto-Classification
    // ------------------------------------------------------------------------

    /// Update document classification.
    pub async fn update_classification(
        &self,
        document_id: Uuid,
        predicted_category: &str,
        confidence: f64,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE documents SET
                ai_predicted_category = $2,
                ai_classification_confidence = $3,
                ai_classification_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(predicted_category)
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        // Record in history
        sqlx::query(
            r#"
            INSERT INTO document_classification_history
                (document_id, predicted_category, confidence)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(document_id)
        .bind(predicted_category)
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Submit classification feedback.
    pub async fn submit_classification_feedback(
        &self,
        feedback: ClassificationFeedback,
    ) -> Result<(), SqlxError> {
        // Update the document
        sqlx::query(
            r#"
            UPDATE documents SET
                ai_classification_accepted = $2,
                category = COALESCE($3, category),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(feedback.document_id)
        .bind(feedback.accepted)
        .bind(&feedback.correct_category)
        .execute(&self.pool)
        .await?;

        // Update the latest classification history entry
        sqlx::query(
            r#"
            UPDATE document_classification_history SET
                was_accepted = $2,
                actual_category = $3,
                feedback_by = $4
            WHERE id = (
                SELECT id FROM document_classification_history
                WHERE document_id = $1
                ORDER BY created_at DESC
                LIMIT 1
            )
            "#,
        )
        .bind(feedback.document_id)
        .bind(feedback.accepted)
        .bind(&feedback.correct_category)
        .bind(feedback.feedback_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get classification history for a document.
    pub async fn get_classification_history(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<DocumentClassificationHistory>, SqlxError> {
        sqlx::query_as::<_, DocumentClassificationHistory>(
            r#"
            SELECT * FROM document_classification_history
            WHERE document_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
    }

    // ------------------------------------------------------------------------
    // Story 28.4: Document Summarization
    // ------------------------------------------------------------------------

    /// Queue a document for summarization.
    pub async fn queue_for_summarization(
        &self,
        request: GenerateSummaryRequest,
    ) -> Result<Uuid, SqlxError> {
        let row = sqlx::query(
            r#"
            INSERT INTO document_summarization_queue
                (document_id, priority, requested_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (document_id) DO UPDATE SET
                priority = LEAST(document_summarization_queue.priority, EXCLUDED.priority),
                next_attempt_at = NOW()
            RETURNING id
            "#,
        )
        .bind(request.document_id)
        .bind(request.priority.unwrap_or(5))
        .bind(request.requested_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Get pending summarization queue items.
    pub async fn get_pending_summarization_items(
        &self,
        limit: i64,
    ) -> Result<Vec<DocumentSummarizationQueue>, SqlxError> {
        sqlx::query_as::<_, DocumentSummarizationQueue>(
            r#"
            SELECT * FROM document_summarization_queue
            WHERE attempts < max_attempts
              AND next_attempt_at <= NOW()
            ORDER BY priority, next_attempt_at
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Update document summary.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_summary(
        &self,
        document_id: Uuid,
        summary: &str,
        key_points: Option<serde_json::Value>,
        action_items: Option<serde_json::Value>,
        topics: Option<serde_json::Value>,
        word_count: Option<i32>,
        language: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE documents SET
                ai_summary = $2,
                ai_key_points = COALESCE($3, ai_key_points),
                ai_action_items = COALESCE($4, ai_action_items),
                ai_topics = COALESCE($5, ai_topics),
                word_count = COALESCE($6, word_count),
                language_detected = COALESCE($7, language_detected),
                ai_summary_generated_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(summary)
        .bind(key_points)
        .bind(action_items)
        .bind(topics)
        .bind(word_count)
        .bind(language)
        .execute(&self.pool)
        .await?;

        // Remove from queue
        sqlx::query("DELETE FROM document_summarization_queue WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Statistics
    // ------------------------------------------------------------------------

    /// Get or create daily statistics for an organization.
    pub async fn get_or_create_daily_stats(
        &self,
        organization_id: Uuid,
        date: NaiveDate,
    ) -> Result<DocumentIntelligenceStats, SqlxError> {
        // Try to insert first
        let result = sqlx::query_as::<_, DocumentIntelligenceStats>(
            r#"
            INSERT INTO document_intelligence_stats (organization_id, date)
            VALUES ($1, $2)
            ON CONFLICT (organization_id, date) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        // If insert returned nothing (conflict), fetch existing
        match result {
            Some(stats) => Ok(stats),
            None => {
                sqlx::query_as::<_, DocumentIntelligenceStats>(
                    r#"
                    SELECT * FROM document_intelligence_stats
                    WHERE organization_id = $1 AND date = $2
                    "#,
                )
                .bind(organization_id)
                .bind(date)
                .fetch_one(&self.pool)
                .await
            }
        }
    }

    /// Increment OCR completed count.
    pub async fn increment_ocr_stats(
        &self,
        organization_id: Uuid,
        success: bool,
        pages: i32,
        confidence: Option<f64>,
    ) -> Result<(), SqlxError> {
        let today = Utc::now().date_naive();

        if success {
            sqlx::query(
                r#"
                INSERT INTO document_intelligence_stats
                    (organization_id, date, documents_processed, ocr_completed, total_pages_processed, avg_ocr_confidence)
                VALUES ($1, $2, 1, 1, $3, $4)
                ON CONFLICT (organization_id, date) DO UPDATE SET
                    documents_processed = document_intelligence_stats.documents_processed + 1,
                    ocr_completed = document_intelligence_stats.ocr_completed + 1,
                    total_pages_processed = document_intelligence_stats.total_pages_processed + EXCLUDED.total_pages_processed,
                    avg_ocr_confidence = (
                        COALESCE(document_intelligence_stats.avg_ocr_confidence, 0) * document_intelligence_stats.ocr_completed
                        + COALESCE(EXCLUDED.avg_ocr_confidence, 0)
                    ) / (document_intelligence_stats.ocr_completed + 1),
                    updated_at = NOW()
                "#,
            )
            .bind(organization_id)
            .bind(today)
            .bind(pages)
            .bind(confidence)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO document_intelligence_stats
                    (organization_id, date, documents_processed, ocr_failed)
                VALUES ($1, $2, 1, 1)
                ON CONFLICT (organization_id, date) DO UPDATE SET
                    documents_processed = document_intelligence_stats.documents_processed + 1,
                    ocr_failed = document_intelligence_stats.ocr_failed + 1,
                    updated_at = NOW()
                "#,
            )
            .bind(organization_id)
            .bind(today)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Increment classification stats.
    pub async fn increment_classification_stats(
        &self,
        organization_id: Uuid,
        accepted: bool,
        confidence: f64,
    ) -> Result<(), SqlxError> {
        let today = Utc::now().date_naive();

        sqlx::query(
            r#"
            INSERT INTO document_intelligence_stats
                (organization_id, date, classifications_completed, classifications_accepted, avg_classification_confidence)
            VALUES ($1, $2, 1, $3::int, $4)
            ON CONFLICT (organization_id, date) DO UPDATE SET
                classifications_completed = document_intelligence_stats.classifications_completed + 1,
                classifications_accepted = document_intelligence_stats.classifications_accepted + EXCLUDED.classifications_accepted,
                avg_classification_confidence = (
                    COALESCE(document_intelligence_stats.avg_classification_confidence, 0) * document_intelligence_stats.classifications_completed
                    + EXCLUDED.avg_classification_confidence
                ) / (document_intelligence_stats.classifications_completed + 1),
                updated_at = NOW()
            "#,
        )
        .bind(organization_id)
        .bind(today)
        .bind(if accepted { 1 } else { 0 })
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Increment summarization stats.
    pub async fn increment_summarization_stats(
        &self,
        organization_id: Uuid,
    ) -> Result<(), SqlxError> {
        let today = Utc::now().date_naive();

        sqlx::query(
            r#"
            INSERT INTO document_intelligence_stats
                (organization_id, date, summaries_generated)
            VALUES ($1, $2, 1)
            ON CONFLICT (organization_id, date) DO UPDATE SET
                summaries_generated = document_intelligence_stats.summaries_generated + 1,
                updated_at = NOW()
            "#,
        )
        .bind(organization_id)
        .bind(today)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get intelligence stats for a date range.
    pub async fn get_intelligence_stats(
        &self,
        organization_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DocumentIntelligenceStats>, SqlxError> {
        sqlx::query_as::<_, DocumentIntelligenceStats>(
            r#"
            SELECT * FROM document_intelligence_stats
            WHERE organization_id = $1
              AND date >= $2
              AND date <= $3
            ORDER BY date DESC
            "#,
        )
        .bind(organization_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await
    }

    /// Get extracted text for a document (Epic 92).
    pub async fn get_extracted_text(&self, document_id: Uuid) -> Result<Option<String>, SqlxError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT extracted_text FROM documents
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(text,)| text))
    }

    /// Get extracted text for a document on an RLS-enabled connection (Epic 92).
    ///
    /// RLS-aware counterpart of [`Self::get_extracted_text`]. `documents` is
    /// under FORCE ROW LEVEL SECURITY (#754), so the read must run on the
    /// caller's org-scoped connection.
    pub async fn get_extracted_text_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<Option<String>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT extracted_text FROM documents
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(document_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.and_then(|(text,)| text))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build folder tree from flat list.
fn build_folder_tree(nodes: Vec<FolderTreeNode>) -> Vec<FolderTreeNode> {
    use std::collections::HashMap;

    let mut node_map: HashMap<Uuid, FolderTreeNode> = HashMap::new();
    let mut root_ids: Vec<Uuid> = Vec::new();

    // First pass: create all nodes
    for node in nodes {
        if node.parent_id.is_none() {
            root_ids.push(node.id);
        }
        node_map.insert(node.id, node);
    }

    // Second pass: build parent-child relationships
    let mut children_map: HashMap<Uuid, Vec<FolderTreeNode>> = HashMap::new();
    for node in node_map.values() {
        if let Some(parent_id) = node.parent_id {
            children_map
                .entry(parent_id)
                .or_default()
                .push(node.clone());
        }
    }

    // Third pass: attach children to parents
    fn attach_children(
        node: &mut FolderTreeNode,
        children_map: &HashMap<Uuid, Vec<FolderTreeNode>>,
    ) {
        if let Some(children) = children_map.get(&node.id) {
            let mut child_nodes: Vec<FolderTreeNode> = children.clone();
            for child in &mut child_nodes {
                attach_children(child, children_map);
            }
            node.children = Some(child_nodes);
        }
    }

    root_ids
        .iter()
        .filter_map(|id| {
            node_map.get(id).cloned().map(|mut node| {
                attach_children(&mut node, &children_map);
                node
            })
        })
        .collect()
}

/// Generate a secure random share token.
///
/// Uses OS CSPRNG directly rather than `thread_rng`.
fn generate_share_token() -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rand_core::UnwrapErr(rand::rngs::SysRng);
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hash a password using Argon2.
fn hash_password(password: &str) -> Result<String, SqlxError> {
    use argon2::{password_hash::PasswordHasher, Argon2};
    let argon2 = Argon2::default();
    // password-hash 0.6: `hash_password` generates a random salt internally
    // (via the `getrandom` feature) — no explicit `SaltString` needed.
    argon2
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| {
            tracing::error!("Failed to hash password: {}", e);
            SqlxError::Protocol("Password hashing failed".to_string())
        })
}

/// Verify a password against a hash.
fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{phc::PasswordHash, PasswordVerifier},
        Argon2,
    };
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

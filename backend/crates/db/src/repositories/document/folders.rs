//! Folder operations (Story 7A.2) — RLS-aware + legacy variants.

use super::internal::build_folder_tree;
use super::DocumentRepository;
use crate::models::{CreateFolder, DocumentFolder, FolderTreeNode, FolderWithCount, UpdateFolder};
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

impl DocumentRepository {
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
                -- parent_id is tri-state (#1589): $4 = "the field was provided",
                -- $5 = the new value (NULL detaches to root). A plain
                -- COALESCE($5, parent_id) could never set NULL, so a move to
                -- root was silently ignored.
                parent_id = CASE WHEN $4 THEN $5 ELSE parent_id END,
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.parent_id.is_some())
        .bind(data.parent_id.flatten())
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
}

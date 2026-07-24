//! Document versioning operations (Story 7B.1) — RLS-aware + legacy variants.

use super::DocumentRepository;
use crate::models::{CreateDocumentVersion, Document, DocumentVersion, DocumentVersionHistory};
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

impl DocumentRepository {
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
            RETURNING
                id, organization_id, folder_id, title, description,
                category::text AS category, file_key, file_name, mime_type,
                size_bytes, access_scope::text AS access_scope,
                access_target_ids, access_roles, created_by, created_at,
                updated_at, deleted_at, version_number, parent_document_id,
                is_current_version, template_id, generation_metadata
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
            RETURNING
                id, organization_id, folder_id, title, description,
                category::text AS category, file_key, file_name, mime_type,
                size_bytes, access_scope::text AS access_scope,
                access_target_ids, access_roles, created_by, created_at,
                updated_at, deleted_at, version_number, parent_document_id,
                is_current_version, template_id, generation_metadata
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
}

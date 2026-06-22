//! Share operations (Story 7A.5) — RLS-aware + legacy variants.

use super::internal::{generate_share_token, hash_password, verify_password};
use super::DocumentRepository;
use crate::models::{
    CreateShare, DocumentShare, LogShareAccess, ShareAccessLog, ShareWithDocument,
};
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

impl DocumentRepository {
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
                u.name as shared_by_name
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
}

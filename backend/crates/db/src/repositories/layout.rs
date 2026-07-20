use crate::models::layout::{
    LayoutConfigRow, LayoutConfigVersionRow, LayoutKillFlagRow, LayoutRegistryManifestRow,
    LayoutTenantOverrideRow,
};
use sqlx::{Error as SqlxError, Executor, PgConnection, Postgres};
use uuid::Uuid;

/// Failure modes of [`LayoutRepository::publish`].
///
/// `DraftChanged` is the optimistic-concurrency signal: the draft the caller
/// validated no longer matches the stored draft (a concurrent draft PUT won
/// the race), so publishing it would ship an unvalidated config. Callers
/// should surface this as a retryable conflict (HTTP 409).
#[derive(Debug, thiserror::Error)]
pub enum LayoutPublishError {
    #[error("unknown screen")]
    ScreenNotFound,
    #[error("draft changed during publish")]
    DraftChanged,
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
}

/// Stateless repository for the layout control plane. Global tables
/// (configs/versions/kills/manifests) are platform-admin-owned and carry no
/// RLS — call them with the unscoped pool. `layout_tenant_overrides` is
/// FORCE-RLS org-scoped — call those methods with an RLS-contexted executor.
#[derive(Clone, Default)]
pub struct LayoutRepository;

impl LayoutRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_config<'e, E>(
        &self,
        executor: E,
        screen: &str,
    ) -> Result<Option<LayoutConfigRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "SELECT id, screen, draft, published, published_version, rails, updated_by,
                    created_at, updated_at
             FROM layout_configs WHERE screen = $1",
        )
        .bind(screen)
        .fetch_optional(executor)
        .await
    }

    pub async fn list_configs<'e, E>(&self, executor: E) -> Result<Vec<LayoutConfigRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "SELECT id, screen, draft, published, published_version, rails, updated_by,
                    created_at, updated_at
             FROM layout_configs ORDER BY screen",
        )
        .fetch_all(executor)
        .await
    }

    pub async fn upsert_draft<'e, E>(
        &self,
        executor: E,
        screen: &str,
        draft: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "INSERT INTO layout_configs (screen, draft, updated_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (screen)
             DO UPDATE SET draft = EXCLUDED.draft, updated_by = EXCLUDED.updated_by,
                           updated_at = now()
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(draft)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }

    pub async fn set_rails<'e, E>(
        &self,
        executor: E,
        screen: &str,
        rails: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigRow>(
            "INSERT INTO layout_configs (screen, rails, updated_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (screen)
             DO UPDATE SET rails = EXCLUDED.rails, updated_by = EXCLUDED.updated_by,
                           updated_at = now()
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(rails)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }

    /// Publish the current draft: draft → published, version += 1, snapshot the
    /// version row. Runs in a transaction on the given connection.
    /// Kill flags are intentionally untouched (spec §5).
    ///
    /// TOCTOU guard: `expected_draft` is the draft value the caller validated.
    /// The UPDATE only fires when the stored draft still equals it (jsonb
    /// equality); if the screen exists but the draft changed concurrently, the
    /// caller gets [`LayoutPublishError::DraftChanged`] and must re-validate.
    pub async fn publish(
        &self,
        conn: &mut PgConnection,
        screen: &str,
        expected_draft: &serde_json::Value,
        published_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, LayoutPublishError> {
        let mut tx = sqlx::Connection::begin(conn).await?;
        let row = sqlx::query_as::<_, LayoutConfigRow>(
            "UPDATE layout_configs
             SET published = draft,
                 published_version = published_version + 1,
                 updated_by = $2,
                 updated_at = now()
             WHERE screen = $1 AND draft = $3
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(published_by)
        .bind(expected_draft)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            // 0 rows: either the screen doesn't exist (404) or the draft
            // changed under us (409). Disambiguate inside the same tx.
            let exists =
                sqlx::query_scalar::<_, i32>("SELECT 1 FROM layout_configs WHERE screen = $1")
                    .bind(screen)
                    .fetch_optional(&mut *tx)
                    .await?
                    .is_some();
            return Err(if exists {
                LayoutPublishError::DraftChanged
            } else {
                LayoutPublishError::ScreenNotFound
            });
        };
        sqlx::query(
            "INSERT INTO layout_config_versions (screen, version, config, published_by)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(screen)
        .bind(row.published_version)
        .bind(row.published.as_ref())
        .bind(published_by)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Roll back to a prior version: that version's config becomes both the
    /// published config and the draft, recorded as a NEW version (immutable,
    /// monotonically increasing history). Kill flags untouched (spec §5).
    pub async fn rollback(
        &self,
        conn: &mut PgConnection,
        screen: &str,
        to_version: i32,
        published_by: Option<Uuid>,
    ) -> Result<LayoutConfigRow, SqlxError> {
        let mut tx = sqlx::Connection::begin(conn).await?;
        let target = sqlx::query_as::<_, LayoutConfigVersionRow>(
            "SELECT id, screen, version, config, published_by, published_at
             FROM layout_config_versions WHERE screen = $1 AND version = $2",
        )
        .bind(screen)
        .bind(to_version)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
        let row = sqlx::query_as::<_, LayoutConfigRow>(
            "UPDATE layout_configs
             SET published = $2,
                 draft = $2,
                 published_version = published_version + 1,
                 updated_by = $3,
                 updated_at = now()
             WHERE screen = $1
             RETURNING id, screen, draft, published, published_version, rails, updated_by,
                       created_at, updated_at",
        )
        .bind(screen)
        .bind(&target.config)
        .bind(published_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SqlxError::RowNotFound)?;
        sqlx::query(
            "INSERT INTO layout_config_versions (screen, version, config, published_by)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(screen)
        .bind(row.published_version)
        .bind(&target.config)
        .bind(published_by)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row)
    }

    pub async fn list_versions<'e, E>(
        &self,
        executor: E,
        screen: &str,
    ) -> Result<Vec<LayoutConfigVersionRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutConfigVersionRow>(
            "SELECT id, screen, version, config, published_by, published_at
             FROM layout_config_versions WHERE screen = $1 ORDER BY version DESC",
        )
        .bind(screen)
        .fetch_all(executor)
        .await
    }

    pub async fn kill<'e, E>(
        &self,
        executor: E,
        screen: &str,
        section_type: &str,
        killed_by: Option<Uuid>,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            "INSERT INTO layout_kill_flags (screen, section_type, killed_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (screen, section_type) DO NOTHING",
        )
        .bind(screen)
        .bind(section_type)
        .bind(killed_by)
        .execute(executor)
        .await
        .map(|_| ())
    }

    pub async fn unkill<'e, E>(
        &self,
        executor: E,
        screen: &str,
        section_type: &str,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let res =
            sqlx::query("DELETE FROM layout_kill_flags WHERE screen = $1 AND section_type = $2")
                .bind(screen)
                .bind(section_type)
                .execute(executor)
                .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list_kills<'e, E>(
        &self,
        executor: E,
        screen: &str,
    ) -> Result<Vec<LayoutKillFlagRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutKillFlagRow>(
            "SELECT screen, section_type, killed_by, killed_at
             FROM layout_kill_flags WHERE screen = $1 ORDER BY section_type",
        )
        .bind(screen)
        .fetch_all(executor)
        .await
    }

    pub async fn upsert_manifest<'e, E>(
        &self,
        executor: E,
        platform: &str,
        manifest: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutRegistryManifestRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutRegistryManifestRow>(
            "INSERT INTO layout_registry_manifests (platform, manifest, updated_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (platform)
             DO UPDATE SET manifest = EXCLUDED.manifest, updated_by = EXCLUDED.updated_by,
                           updated_at = now()
             RETURNING platform, manifest, updated_by, updated_at",
        )
        .bind(platform)
        .bind(manifest)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }

    pub async fn get_manifest<'e, E>(
        &self,
        executor: E,
        platform: &str,
    ) -> Result<Option<LayoutRegistryManifestRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutRegistryManifestRow>(
            "SELECT platform, manifest, updated_by, updated_at
             FROM layout_registry_manifests WHERE platform = $1",
        )
        .bind(platform)
        .fetch_optional(executor)
        .await
    }

    pub async fn list_manifests<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<LayoutRegistryManifestRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutRegistryManifestRow>(
            "SELECT platform, manifest, updated_by, updated_at
             FROM layout_registry_manifests ORDER BY platform",
        )
        .fetch_all(executor)
        .await
    }

    pub async fn get_tenant_override<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        screen: &str,
    ) -> Result<Option<LayoutTenantOverrideRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutTenantOverrideRow>(
            "SELECT id, organization_id, screen, override_config, updated_by,
                    created_at, updated_at
             FROM layout_tenant_overrides
             WHERE organization_id = $1 AND screen = $2",
        )
        .bind(organization_id)
        .bind(screen)
        .fetch_optional(executor)
        .await
    }

    pub async fn upsert_tenant_override<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        screen: &str,
        override_config: &serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<LayoutTenantOverrideRow, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LayoutTenantOverrideRow>(
            "INSERT INTO layout_tenant_overrides (organization_id, screen, override_config, updated_by)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (organization_id, screen)
             DO UPDATE SET override_config = EXCLUDED.override_config,
                           updated_by = EXCLUDED.updated_by, updated_at = now()
             RETURNING id, organization_id, screen, override_config, updated_by,
                       created_at, updated_at",
        )
        .bind(organization_id)
        .bind(screen)
        .bind(override_config)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }
}

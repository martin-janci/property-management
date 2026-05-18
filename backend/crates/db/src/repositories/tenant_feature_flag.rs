//! Per-tenant feature flag repository (Phase 3, defends leak #22).

use crate::models::tenant_feature_flag::{TenantFeatureFlag, UpsertTenantFeatureFlag};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

#[derive(Clone)]
pub struct TenantFeatureFlagRepository {
    pool: DbPool,
}

impl TenantFeatureFlagRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List all per-tenant flags for `organization_id` on a SYSTEM connection.
    /// Used by `/tenant-config` (host-resolved, pre-auth).
    pub async fn list_by_organization_system(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<TenantFeatureFlag>, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = sqlx::query_as::<_, TenantFeatureFlag>(
            r#"
            SELECT organization_id, flag_key, enabled, value, updated_at, updated_by
            FROM tenant_feature_flags
            WHERE organization_id = $1
            ORDER BY flag_key ASC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&mut *conn)
        .await;

        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::warn!(
                error = %e,
                "Failed to clear system RLS context after list_by_organization_system"
            );
        }

        result
    }

    /// List flags via an existing RLS executor (admin endpoints).
    pub async fn list_by_organization_rls<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<TenantFeatureFlag>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, TenantFeatureFlag>(
            r#"
            SELECT organization_id, flag_key, enabled, value, updated_at, updated_by
            FROM tenant_feature_flags
            WHERE organization_id = $1
            ORDER BY flag_key ASC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
    }

    /// Upsert one flag for `organization_id`. Caller must already hold an
    /// RLS context that authorizes writing this org's flags.
    pub async fn upsert_rls<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        updated_by: Option<Uuid>,
        upsert: UpsertTenantFeatureFlag,
    ) -> Result<TenantFeatureFlag, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, TenantFeatureFlag>(
            r#"
            INSERT INTO tenant_feature_flags
                (organization_id, flag_key, enabled, value, updated_by)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (organization_id, flag_key) DO UPDATE SET
                enabled    = EXCLUDED.enabled,
                value      = EXCLUDED.value,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            RETURNING organization_id, flag_key, enabled, value, updated_at, updated_by
            "#,
        )
        .bind(organization_id)
        .bind(&upsert.flag_key)
        .bind(upsert.enabled)
        .bind(&upsert.value)
        .bind(updated_by)
        .fetch_one(executor)
        .await
    }
}

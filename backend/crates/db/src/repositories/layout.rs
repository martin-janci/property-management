use crate::models::layout::LayoutConfigRow;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

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
}

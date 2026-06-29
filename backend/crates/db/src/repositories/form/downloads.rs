//! Form download tracking.

use super::FormRepository;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl FormRepository {
    /// Records a form download.
    pub async fn record_download<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        form_id: Uuid,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO form_downloads (form_id, downloaded_by, ip_address, user_agent)
            SELECT f.id, $3, $4::inet, $5
            FROM forms f
            WHERE f.id = $1
              AND f.organization_id = $2
              AND f.deleted_at IS NULL
            RETURNING form_id
            "#,
        )
        .bind(form_id)
        .bind(org_id)
        .bind(user_id)
        .bind(&ip_address)
        .bind(&user_agent)
        .fetch_optional(executor)
        .await?;

        Ok(inserted.is_some())
    }

    /// Gets download count for a form.
    pub async fn get_download_count<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM form_downloads WHERE form_id = $1")
            .bind(form_id)
            .fetch_one(executor)
            .await
    }
}

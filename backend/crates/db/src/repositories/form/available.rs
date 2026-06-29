//! Published forms available to a user.

use super::FormRepository;
use crate::models::FormSummary;
use sqlx::{Executor, Postgres, Row};
use uuid::Uuid;

impl FormRepository {
    /// Lists published forms available to a user.
    pub async fn list_available_forms<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        building_id: Option<Uuid>,
        _user_role: &str,
    ) -> Result<Vec<FormSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query(
            r#"
            SELECT
                f.id,
                f.title,
                f.description,
                f.category,
                f.status::text AS status,
                f.target_type,
                f.require_signatures,
                f.submission_deadline,
                f.published_at,
                f.created_at,
                COALESCE(
                    (SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id),
                    0
                ) as submission_count,
                u.name as created_by_name
            FROM forms f
            LEFT JOIN users u ON u.id = f.created_by
            WHERE f.organization_id = $1
                AND f.status = 'published'
                AND f.deleted_at IS NULL
                AND (
                    f.target_type = 'all'
                    OR ($2::uuid IS NULL OR f.building_id = $2)
                    OR (f.target_type = 'building' AND $2::uuid = ANY(
                        SELECT jsonb_array_elements_text(f.target_ids)::uuid
                    ))
                )
                AND (f.submission_deadline IS NULL OR f.submission_deadline > NOW())
            ORDER BY f.published_at DESC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(executor)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| FormSummary {
                id: r.get("id"),
                title: r.get("title"),
                description: r.get("description"),
                category: r.get("category"),
                status: r.get("status"),
                target_type: r.get("target_type"),
                require_signatures: r.get("require_signatures"),
                submission_deadline: r.get("submission_deadline"),
                published_at: r.get("published_at"),
                created_at: r.get("created_at"),
                submission_count: r.get("submission_count"),
                created_by_name: r.get("created_by_name"),
            })
            .collect())
    }

    /// Checks if a user has already submitted a form.
    pub async fn has_user_submitted<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM form_submissions WHERE form_id = $1 AND submitted_by = $2",
        )
        .bind(form_id)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(count > 0)
    }
}

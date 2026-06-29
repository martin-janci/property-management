//! Organization form statistics.

use super::FormRepository;
use crate::models::FormStatistics;
use sqlx::{Executor, Postgres, Row};
use uuid::Uuid;

impl FormRepository {
    /// Gets form statistics for an organization.
    pub async fn get_statistics<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<FormStatistics, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM forms WHERE organization_id = $1 AND deleted_at IS NULL) as total_forms,
                (SELECT COUNT(*) FROM forms WHERE organization_id = $1 AND status = 'published' AND deleted_at IS NULL) as published_forms,
                (SELECT COUNT(*) FROM forms WHERE organization_id = $1 AND status = 'draft' AND deleted_at IS NULL) as draft_forms,
                (SELECT COUNT(*) FROM forms WHERE organization_id = $1 AND status = 'archived' AND deleted_at IS NULL) as archived_forms,
                (SELECT COUNT(*) FROM form_submissions WHERE organization_id = $1) as total_submissions,
                (SELECT COUNT(*) FROM form_submissions WHERE organization_id = $1 AND status = 'pending') as pending_submissions,
                (SELECT COUNT(*) FROM form_submissions WHERE organization_id = $1 AND status = 'approved') as approved_submissions,
                (SELECT COUNT(*) FROM form_submissions WHERE organization_id = $1 AND status = 'rejected') as rejected_submissions
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        Ok(FormStatistics {
            total_forms: row.get("total_forms"),
            published_forms: row.get("published_forms"),
            draft_forms: row.get("draft_forms"),
            archived_forms: row.get("archived_forms"),
            total_submissions: row.get("total_submissions"),
            pending_submissions: row.get("pending_submissions"),
            approved_submissions: row.get("approved_submissions"),
            rejected_submissions: row.get("rejected_submissions"),
        })
    }
}

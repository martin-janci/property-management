//! Form submission create, get, list & review.

use super::FormRepository;
use crate::models::{
    form::FormSubmissionParams, submission_status, FormSubmission, FormSubmissionSummary,
    FormSubmissionWithDetails, ReviewSubmission, SubmissionListQuery,
};
use sqlx::{Executor, PgConnection, Postgres, Row};
use uuid::Uuid;

impl FormRepository {
    /// Submits a form.
    pub async fn submit<'e, E>(
        &self,
        executor: E,
        params: FormSubmissionParams,
    ) -> Result<FormSubmission, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let attachments = params
            .data
            .attachments
            .map(|a| serde_json::to_value(a).unwrap_or_default())
            .unwrap_or_else(|| serde_json::json!([]));

        let signature_data = params
            .data
            .signature_data
            .map(|s| serde_json::to_value(s).unwrap_or_default());

        sqlx::query_as::<_, FormSubmission>(
            r#"
            INSERT INTO form_submissions (
                form_id, organization_id, building_id, unit_id,
                submitted_by, data, attachments, signature_data,
                status, ip_address, user_agent
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::form_submission_status, $10::inet, $11)
            RETURNING
                id, form_id, organization_id, building_id, unit_id,
                submitted_by, submitted_at, data, attachments, signature_data,
                status::text AS status,
                reviewed_by, reviewed_at, review_notes,
                ip_address::text AS ip_address,
                user_agent, created_at, updated_at
            "#,
        )
        .bind(params.form_id)
        .bind(params.org_id)
        .bind(params.building_id)
        .bind(params.unit_id)
        .bind(params.user_id)
        .bind(&params.data.data)
        .bind(&attachments)
        .bind(&signature_data)
        .bind(submission_status::PENDING)
        .bind(&params.ip_address)
        .bind(&params.user_agent)
        .fetch_one(executor)
        .await
    }

    /// Gets a submission by ID.
    pub async fn get_submission<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        submission_id: Uuid,
    ) -> Result<Option<FormSubmissionWithDetails>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT
                s.*,
                -- `s.*` returns `status` as the `form_submission_status` ENUM,
                -- which the model reads into a `String` (ColumnDecode). Re-expose
                -- it as text under a distinct name and read that below.
                s.status::text AS submission_status,
                f.title as form_title,
                u.name as submitted_by_name,
                r.name as reviewed_by_name,
                un.designation AS unit_number,
                b.name as building_name
            FROM form_submissions s
            JOIN forms f ON f.id = s.form_id
            JOIN users u ON u.id = s.submitted_by
            LEFT JOIN users r ON r.id = s.reviewed_by
            LEFT JOIN units un ON un.id = s.unit_id
            LEFT JOIN buildings b ON b.id = s.building_id
            WHERE s.id = $1 AND s.organization_id = $2
            "#,
        )
        .bind(submission_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|r| FormSubmissionWithDetails {
            submission: FormSubmission {
                id: r.get("id"),
                form_id: r.get("form_id"),
                organization_id: r.get("organization_id"),
                building_id: r.get("building_id"),
                unit_id: r.get("unit_id"),
                submitted_by: r.get("submitted_by"),
                submitted_at: r.get("submitted_at"),
                data: r.get("data"),
                attachments: r.get("attachments"),
                signature_data: r.get("signature_data"),
                status: r.get("submission_status"),
                reviewed_by: r.get("reviewed_by"),
                reviewed_at: r.get("reviewed_at"),
                review_notes: r.get("review_notes"),
                ip_address: r.get("ip_address"),
                user_agent: r.get("user_agent"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            },
            form_title: r.get("form_title"),
            submitted_by_name: r.get("submitted_by_name"),
            reviewed_by_name: r.get("reviewed_by_name"),
            unit_number: r.get("unit_number"),
            building_name: r.get("building_name"),
        }))
    }

    /// Lists form submissions with filtering and pagination.
    pub async fn list_submissions(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        query: SubmissionListQuery,
    ) -> Result<(Vec<FormSubmissionSummary>, i64), sqlx::Error> {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;

        // Count query
        let mut count_conditions = vec!["s.organization_id = $1"];
        if query.form_id.is_some() {
            count_conditions.push("s.form_id = $2");
        }
        if query.status.is_some() {
            count_conditions.push("s.status::text = $3");
        }

        let count_where = count_conditions.join(" AND ");
        let count_sql = format!(
            "SELECT COUNT(*) FROM form_submissions s WHERE {}",
            count_where
        );

        let mut count_query =
            sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_sql)).bind(org_id);
        if let Some(ref form_id) = query.form_id {
            count_query = count_query.bind(form_id);
        }
        if let Some(ref status) = query.status {
            count_query = count_query.bind(status);
        }

        let total = count_query.fetch_one(&mut *conn).await?;

        // Main query
        let rows = sqlx::query(
            r#"
            SELECT
                s.id,
                s.form_id,
                f.title as form_title,
                s.submitted_by,
                u.name as submitted_by_name,
                s.submitted_at,
                s.status::text AS status,
                s.signature_data IS NOT NULL as has_signature,
                un.designation AS unit_number,
                b.name as building_name
            FROM form_submissions s
            JOIN forms f ON f.id = s.form_id
            JOIN users u ON u.id = s.submitted_by
            LEFT JOIN units un ON un.id = s.unit_id
            LEFT JOIN buildings b ON b.id = s.building_id
            WHERE s.organization_id = $1
                AND ($2::uuid IS NULL OR s.form_id = $2)
                AND ($3::text IS NULL OR s.status::text = $3)
                AND ($4::uuid IS NULL OR s.building_id = $4)
                AND ($5::uuid IS NULL OR s.unit_id = $5)
                AND ($6::uuid IS NULL OR s.submitted_by = $6)
                AND ($7::timestamptz IS NULL OR s.submitted_at >= $7)
                AND ($8::timestamptz IS NULL OR s.submitted_at <= $8)
            ORDER BY s.submitted_at DESC
            LIMIT $9 OFFSET $10
            "#,
        )
        .bind(org_id)
        .bind(query.form_id)
        .bind(&query.status)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.submitted_by)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await?;

        let submissions: Vec<FormSubmissionSummary> = rows
            .into_iter()
            .map(|r| FormSubmissionSummary {
                id: r.get("id"),
                form_id: r.get("form_id"),
                form_title: r.get("form_title"),
                submitted_by: r.get("submitted_by"),
                submitted_by_name: r.get("submitted_by_name"),
                submitted_at: r.get("submitted_at"),
                status: r.get("status"),
                has_signature: r.get("has_signature"),
                unit_number: r.get("unit_number"),
                building_name: r.get("building_name"),
            })
            .collect();

        Ok((submissions, total))
    }

    /// Reviews a submission (approve/reject).
    pub async fn review_submission<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        submission_id: Uuid,
        reviewer_id: Uuid,
        data: ReviewSubmission,
    ) -> Result<FormSubmission, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, FormSubmission>(
            r#"
            UPDATE form_submissions SET
                status = $1::form_submission_status,
                reviewed_by = $2,
                reviewed_at = NOW(),
                review_notes = $3,
                updated_at = NOW()
            WHERE id = $4 AND organization_id = $5
            RETURNING
                id, form_id, organization_id, building_id, unit_id,
                submitted_by, submitted_at, data, attachments, signature_data,
                status::text AS status,
                reviewed_by, reviewed_at, review_notes,
                ip_address::text AS ip_address,
                user_agent, created_at, updated_at
            "#,
        )
        .bind(&data.status)
        .bind(reviewer_id)
        .bind(&data.review_notes)
        .bind(submission_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }
}

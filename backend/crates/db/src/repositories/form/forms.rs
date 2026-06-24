//! Form CRUD, listing, publish & archive.

use super::FormRepository;
use crate::models::{
    form_status, CreateForm, Form, FormListQuery, FormSummary, FormWithDetails, UpdateForm,
};
use sqlx::{Executor, PgConnection, Postgres, Row};
use uuid::Uuid;

impl FormRepository {
    /// Creates a new form.
    pub async fn create(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateForm,
    ) -> Result<Form, sqlx::Error> {
        let target_ids = data
            .target_ids
            .map(|ids| serde_json::json!(ids))
            .unwrap_or_else(|| serde_json::json!([]));

        let form = sqlx::query_as::<_, Form>(
            r#"
            INSERT INTO forms (
                organization_id, building_id, title, description, category,
                status, target_type, target_ids, require_signatures,
                allow_multiple_submissions, submission_deadline, confirmation_message,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6::form_status, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, organization_id, building_id, title, description, category,
                status::text AS status, version, target_type, target_ids, require_signatures,
                allow_multiple_submissions, submission_deadline, confirmation_message,
                pdf_template_settings, created_by, updated_by, published_by, published_at,
                archived_at, created_at, updated_at, deleted_at
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.category)
        .bind(form_status::DRAFT)
        .bind(data.target_type.as_deref().unwrap_or("all"))
        .bind(&target_ids)
        .bind(data.require_signatures)
        .bind(data.allow_multiple_submissions)
        .bind(data.submission_deadline)
        .bind(&data.confirmation_message)
        .bind(user_id)
        .fetch_one(&mut *conn)
        .await?;

        // Create fields if provided
        // Note: Batch insert was attempted but caused lifetime issues with JSON values.
        // The performance benefit is minimal for typical form creation (< 20 fields).
        // Keeping the simple approach for maintainability.
        for (index, field) in data.fields.into_iter().enumerate() {
            self.create_field(&mut *conn, form.id, field, index as i32)
                .await?;
        }

        Ok(form)
    }

    /// Gets a form by ID.
    pub async fn get<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        form_id: Uuid,
    ) -> Result<Option<Form>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Form>(
            r#"
            SELECT
                id, organization_id, building_id, title, description, category,
                status::text AS status, version, target_type, target_ids,
                require_signatures, allow_multiple_submissions, submission_deadline,
                confirmation_message, pdf_template_settings, created_by, updated_by,
                published_by, published_at, archived_at, created_at, updated_at,
                deleted_at
            FROM forms
            WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(form_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Gets a form with all its details.
    pub async fn get_with_details(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        form_id: Uuid,
    ) -> Result<Option<FormWithDetails>, sqlx::Error> {
        let form = match self.get(&mut *conn, org_id, form_id).await? {
            Some(f) => f,
            None => return Ok(None),
        };

        let fields = self.get_fields(&mut *conn, form_id).await?;

        let submission_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM form_submissions WHERE form_id = $1",
        )
        .bind(form_id)
        .fetch_one(&mut *conn)
        .await?;

        let created_by_name = sqlx::query_scalar::<_, String>(
            "SELECT COALESCE(name, email) FROM users WHERE id = $1",
        )
        .bind(form.created_by)
        .fetch_optional(&mut *conn)
        .await?;

        let published_by_name = if let Some(published_by) = form.published_by {
            sqlx::query_scalar::<_, String>("SELECT COALESCE(name, email) FROM users WHERE id = $1")
                .bind(published_by)
                .fetch_optional(&mut *conn)
                .await?
        } else {
            None
        };

        Ok(Some(FormWithDetails {
            form,
            fields,
            created_by_name,
            published_by_name,
            submission_count,
        }))
    }

    /// Lists forms for an organization with filtering and pagination.
    pub async fn list(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        query: FormListQuery,
    ) -> Result<(Vec<FormSummary>, i64), sqlx::Error> {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;

        let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
        let sort_order = query.sort_order.as_deref().unwrap_or("DESC");

        // Use parameterized query with NULL checks instead of dynamic SQL
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM forms f
            WHERE f.organization_id = $1
                AND f.deleted_at IS NULL
                AND ($2::text IS NULL OR f.status::text = $2)
                AND ($3::text IS NULL OR f.category = $3)
                AND ($4::uuid IS NULL OR f.building_id = $4)
                AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
            "#,
        )
        .bind(org_id)
        .bind(&query.status)
        .bind(&query.category)
        .bind(query.building_id)
        .bind(query.search.as_ref().map(|s| format!("%{}%", s)))
        .fetch_one(&mut *conn)
        .await?;

        // Build complete SQL with a safe ORDER BY. The SELECT/FROM/WHERE body is
        // identical across all sort variants; only the ORDER BY column+direction
        // differs. We keep the ORDER BY fragments as hardcoded string literals
        // selected by `match` (NO user input ever reaches the SQL — `sort_by` /
        // `sort_order` only choose which literal arm runs), then concatenate them
        // with the shared body. This preserves the previous "avoid format!() with
        // user input" guarantee while removing the 8-way SQL duplication.
        let is_asc = sort_order.to_uppercase() == "ASC";
        let order_by = match (sort_by, is_asc) {
            ("title", true) => "ORDER BY f.title ASC",
            ("title", false) => "ORDER BY f.title DESC",
            ("status", true) => "ORDER BY f.status ASC",
            ("status", false) => "ORDER BY f.status DESC",
            ("published_at", true) => "ORDER BY f.published_at ASC",
            ("published_at", false) => "ORDER BY f.published_at DESC",
            ("category", true) => "ORDER BY f.category ASC",
            ("category", false) => "ORDER BY f.category DESC",
            // Default sort column: created_at
            (_, true) => "ORDER BY f.created_at ASC",
            (_, false) => "ORDER BY f.created_at DESC",
        };

        // The ORDER BY literal above is hardcoded; the only dynamic input is the
        // arm-selection, so this concatenation cannot inject user SQL.
        let sql = format!(
            r#"
            SELECT f.id, f.title, f.description, f.category, f.status::text AS status, f.target_type,
                   f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                   COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                   u.name as created_by_name
            FROM forms f LEFT JOIN users u ON u.id = f.created_by
            WHERE f.organization_id = $1 AND f.deleted_at IS NULL
              AND ($2::text IS NULL OR f.status::text = $2) AND ($3::text IS NULL OR f.category = $3)
              AND ($4::uuid IS NULL OR f.building_id = $4)
              AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
            {order_by} LIMIT $6 OFFSET $7
            "#
        );

        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(org_id)
            .bind(&query.status)
            .bind(&query.category)
            .bind(query.building_id)
            .bind(query.search.as_ref().map(|s| format!("%{}%", s)))
            .bind(per_page)
            .bind(offset)
            .fetch_all(&mut *conn)
            .await?;

        let forms: Vec<FormSummary> = rows
            .into_iter()
            .map(|row| FormSummary {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                category: row.get("category"),
                status: row.get("status"),
                target_type: row.get("target_type"),
                require_signatures: row.get("require_signatures"),
                submission_deadline: row.get("submission_deadline"),
                published_at: row.get("published_at"),
                created_at: row.get("created_at"),
                submission_count: row.get("submission_count"),
                created_by_name: row.get("created_by_name"),
            })
            .collect();

        Ok((forms, total))
    }

    /// Updates a form.
    pub async fn update(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        form_id: Uuid,
        user_id: Uuid,
        data: UpdateForm,
    ) -> Result<Form, sqlx::Error> {
        // Check if form exists and is in draft status
        let existing = self.get(&mut *conn, org_id, form_id).await?;
        if existing.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }

        let target_ids = data
            .target_ids
            .map(|ids| serde_json::json!(ids))
            .unwrap_or_else(|| serde_json::json!([]));

        sqlx::query_as::<_, Form>(
            r#"
            UPDATE forms SET
                title = COALESCE($1, title),
                description = COALESCE($2, description),
                category = COALESCE($3, category),
                building_id = COALESCE($4, building_id),
                target_type = COALESCE($5, target_type),
                target_ids = $6,
                require_signatures = COALESCE($7, require_signatures),
                allow_multiple_submissions = COALESCE($8, allow_multiple_submissions),
                submission_deadline = $9,
                confirmation_message = COALESCE($10, confirmation_message),
                updated_by = $11,
                updated_at = NOW()
            WHERE id = $12 AND organization_id = $13 AND deleted_at IS NULL
            RETURNING id, organization_id, building_id, title, description, category,
                status::text AS status, version, target_type, target_ids, require_signatures,
                allow_multiple_submissions, submission_deadline, confirmation_message,
                pdf_template_settings, created_by, updated_by, published_by, published_at,
                archived_at, created_at, updated_at, deleted_at
            "#,
        )
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.category)
        .bind(data.building_id)
        .bind(&data.target_type)
        .bind(&target_ids)
        .bind(data.require_signatures)
        .bind(data.allow_multiple_submissions)
        .bind(data.submission_deadline)
        .bind(&data.confirmation_message)
        .bind(user_id)
        .bind(form_id)
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await
    }

    /// Soft deletes a form.
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        form_id: Uuid,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE forms SET deleted_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(form_id)
        .bind(org_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Publishes a form.
    pub async fn publish<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        form_id: Uuid,
        user_id: Uuid,
    ) -> Result<Form, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Form>(
            r#"
            UPDATE forms SET
                status = $1::form_status,
                published_by = $2,
                published_at = NOW(),
                updated_at = NOW()
            WHERE id = $3 AND organization_id = $4 AND status = $5::form_status AND deleted_at IS NULL
            RETURNING id, organization_id, building_id, title, description, category,
                status::text AS status, version, target_type, target_ids, require_signatures,
                allow_multiple_submissions, submission_deadline, confirmation_message,
                pdf_template_settings, created_by, updated_by, published_by, published_at,
                archived_at, created_at, updated_at, deleted_at
            "#,
        )
        .bind(form_status::PUBLISHED)
        .bind(user_id)
        .bind(form_id)
        .bind(org_id)
        .bind(form_status::DRAFT)
        .fetch_one(executor)
        .await
    }

    /// Archives a form.
    pub async fn archive<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        form_id: Uuid,
    ) -> Result<Form, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Form>(
            r#"
            UPDATE forms SET
                status = $1::form_status,
                archived_at = NOW(),
                updated_at = NOW()
            WHERE id = $2 AND organization_id = $3 AND deleted_at IS NULL
            RETURNING id, organization_id, building_id, title, description, category,
                status::text AS status, version, target_type, target_ids, require_signatures,
                allow_multiple_submissions, submission_deadline, confirmation_message,
                pdf_template_settings, created_by, updated_by, published_by, published_at,
                archived_at, created_at, updated_at, deleted_at
            "#,
        )
        .bind(form_status::ARCHIVED)
        .bind(form_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }
}

//! Form repository for Epic 54.
//!
//! Handles all database operations for forms, fields, and submissions.
//!
//! # RLS Integration (PAP-67 / PAP-76)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the form tables (`forms`, `form_fields`,
//! `form_submissions`, `form_downloads`). Under `FORCE` the api-server's owner
//! connection is no longer exempt, so a query issued on a connection WITHOUT
//! `app.current_org_id` set collapses to deny-all (own-org reads return empty,
//! writes fail) — and on a BYPASSRLS/superuser pool the same query would run
//! completely unscoped.
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. Multi-statement methods
//! (those that fire more than one query, e.g. a count plus a page fetch, or an
//! INSERT that fans out into child-row INSERTs) take a `&mut PgConnection` so
//! every statement runs on the *same* context-bearing connection. The
//! repository holds **no pool**, so there is no way to issue a query that
//! bypasses RLS. This mirrors the `work_order.rs` (PAP-179) /
//! `budget.rs` (PAP-180) precedent.

use crate::models::{
    form::FormSubmissionParams, form_status, submission_status, CreateForm, CreateFormField, Form,
    FormField, FormListQuery, FormStatistics, FormSubmission, FormSubmissionSummary,
    FormSubmissionWithDetails, FormSummary, FormWithDetails, ReviewSubmission, SubmissionListQuery,
    UpdateForm, UpdateFormField,
};
use sqlx::{Executor, PgConnection, PgPool, Postgres, Row};
use uuid::Uuid;

/// Repository for form-related database operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct FormRepository;

impl FormRepository {
    /// Creates a new form repository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ========================================================================
    // Form CRUD Operations
    // ========================================================================

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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
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
            SELECT * FROM forms
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
                AND ($2::text IS NULL OR f.status = $2)
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

        // Build complete SQL with safe ORDER BY - avoid format!() with user input
        // Use match to select the complete query with hardcoded ORDER BY clause
        let is_asc = sort_order.to_uppercase() == "ASC";
        let sql = match (sort_by, is_asc) {
            ("title", true) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.title ASC LIMIT $6 OFFSET $7
            "#
            }
            ("title", false) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.title DESC LIMIT $6 OFFSET $7
            "#
            }
            ("status", true) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.status ASC LIMIT $6 OFFSET $7
            "#
            }
            ("status", false) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.status DESC LIMIT $6 OFFSET $7
            "#
            }
            ("published_at", true) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.published_at ASC LIMIT $6 OFFSET $7
            "#
            }
            ("published_at", false) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.published_at DESC LIMIT $6 OFFSET $7
            "#
            }
            ("category", true) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.category ASC LIMIT $6 OFFSET $7
            "#
            }
            ("category", false) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.category DESC LIMIT $6 OFFSET $7
            "#
            }
            // Default: created_at DESC
            (_, false) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.created_at DESC LIMIT $6 OFFSET $7
            "#
            }
            // Default: created_at ASC
            (_, true) => {
                r#"
                SELECT f.id, f.title, f.description, f.category, f.status, f.target_type,
                       f.require_signatures, f.submission_deadline, f.published_at, f.created_at,
                       COALESCE((SELECT COUNT(*) FROM form_submissions WHERE form_id = f.id), 0) as submission_count,
                       u.name as created_by_name
                FROM forms f LEFT JOIN users u ON u.id = f.created_by
                WHERE f.organization_id = $1 AND f.deleted_at IS NULL
                  AND ($2::text IS NULL OR f.status = $2) AND ($3::text IS NULL OR f.category = $3)
                  AND ($4::uuid IS NULL OR f.building_id = $4)
                  AND ($5::text IS NULL OR f.title ILIKE $5 OR f.description ILIKE $5)
                ORDER BY f.created_at ASC LIMIT $6 OFFSET $7
            "#
            }
        };

        let rows = sqlx::query(sql)
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
            RETURNING *
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
                status = $1,
                published_by = $2,
                published_at = NOW(),
                updated_at = NOW()
            WHERE id = $3 AND organization_id = $4 AND status = $5 AND deleted_at IS NULL
            RETURNING *
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
                status = $1,
                archived_at = NOW(),
                updated_at = NOW()
            WHERE id = $2 AND organization_id = $3 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(form_status::ARCHIVED)
        .bind(form_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    // ========================================================================
    // Form Field Operations
    // ========================================================================

    /// Creates a new field for a form.
    pub async fn create_field<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        data: CreateFormField,
        order: i32,
    ) -> Result<FormField, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let validation_rules = data
            .validation_rules
            .map(|r| serde_json::to_value(r).unwrap_or_default())
            .unwrap_or_else(|| serde_json::json!({}));

        let options = data
            .options
            .map(|o| serde_json::to_value(o).unwrap_or_default())
            .unwrap_or_else(|| serde_json::json!([]));

        let conditional_display = data
            .conditional_display
            .map(|c| serde_json::to_value(c).unwrap_or_default());

        sqlx::query_as::<_, FormField>(
            r#"
            INSERT INTO form_fields (
                form_id, field_key, label, field_type, required,
                help_text, placeholder, default_value, validation_rules,
                options, field_order, width, section, conditional_display
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(form_id)
        .bind(&data.field_key)
        .bind(&data.label)
        .bind(&data.field_type)
        .bind(data.required)
        .bind(&data.help_text)
        .bind(&data.placeholder)
        .bind(&data.default_value)
        .bind(&validation_rules)
        .bind(&options)
        .bind(if data.field_order > 0 {
            data.field_order
        } else {
            order
        })
        .bind(&data.width)
        .bind(&data.section)
        .bind(&conditional_display)
        .fetch_one(executor)
        .await
    }

    /// Gets all fields for a form.
    pub async fn get_fields<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
    ) -> Result<Vec<FormField>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, FormField>(
            r#"
            SELECT * FROM form_fields
            WHERE form_id = $1
            ORDER BY field_order ASC
            "#,
        )
        .bind(form_id)
        .fetch_all(executor)
        .await
    }

    /// Updates a form field.
    pub async fn update_field<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        field_id: Uuid,
        data: UpdateFormField,
    ) -> Result<FormField, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let validation_rules = data
            .validation_rules
            .map(|r| serde_json::to_value(r).unwrap_or_default());

        let options = data
            .options
            .map(|o| serde_json::to_value(o).unwrap_or_default());

        let conditional_display = data
            .conditional_display
            .map(|c| serde_json::to_value(c).unwrap_or_default());

        sqlx::query_as::<_, FormField>(
            r#"
            UPDATE form_fields SET
                label = COALESCE($1, label),
                field_type = COALESCE($2, field_type),
                required = COALESCE($3, required),
                help_text = COALESCE($4, help_text),
                placeholder = COALESCE($5, placeholder),
                default_value = COALESCE($6, default_value),
                validation_rules = COALESCE($7, validation_rules),
                options = COALESCE($8, options),
                field_order = COALESCE($9, field_order),
                width = COALESCE($10, width),
                section = COALESCE($11, section),
                conditional_display = COALESCE($12, conditional_display),
                updated_at = NOW()
            WHERE id = $13 AND form_id = $14
            RETURNING *
            "#,
        )
        .bind(&data.label)
        .bind(&data.field_type)
        .bind(data.required)
        .bind(&data.help_text)
        .bind(&data.placeholder)
        .bind(&data.default_value)
        .bind(&validation_rules)
        .bind(&options)
        .bind(data.field_order)
        .bind(&data.width)
        .bind(&data.section)
        .bind(&conditional_display)
        .bind(field_id)
        .bind(form_id)
        .fetch_one(executor)
        .await
    }

    /// Deletes a form field.
    pub async fn delete_field<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        field_id: Uuid,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM form_fields WHERE id = $1 AND form_id = $2")
            .bind(field_id)
            .bind(form_id)
            .execute(executor)
            .await?;

        Ok(())
    }

    /// Reorders form fields.
    pub async fn reorder_fields<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        field_orders: Vec<(Uuid, i32)>,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // If there are no fields to reorder, avoid running a no-op query.
        if field_orders.is_empty() {
            return Ok(());
        }

        // Split the (field_id, order) pairs into parallel vectors for efficient bulk update.
        let (field_ids, orders): (Vec<Uuid>, Vec<i32>) = field_orders.into_iter().unzip();

        // Perform a single bulk UPDATE using array parameters and UNNEST.
        sqlx::query(
            r#"
            UPDATE form_fields AS f
            SET field_order = v.field_order,
                updated_at = NOW()
            FROM (
                SELECT
                    UNNEST($1::uuid[]) AS id,
                    UNNEST($2::int4[]) AS field_order
            ) AS v
            WHERE f.form_id = $3
              AND f.id = v.id
            "#,
        )
        .bind(&field_ids)
        .bind(&orders)
        .bind(form_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    // ========================================================================
    // Form Submission Operations
    // ========================================================================

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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::inet, $11)
            RETURNING *
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
                f.title as form_title,
                u.name as submitted_by_name,
                r.name as reviewed_by_name,
                un.unit_number,
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
                status: r.get("status"),
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
            count_conditions.push("s.status = $3");
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
                s.status,
                s.signature_data IS NOT NULL as has_signature,
                un.unit_number,
                b.name as building_name
            FROM form_submissions s
            JOIN forms f ON f.id = s.form_id
            JOIN users u ON u.id = s.submitted_by
            LEFT JOIN units un ON un.id = s.unit_id
            LEFT JOIN buildings b ON b.id = s.building_id
            WHERE s.organization_id = $1
                AND ($2::uuid IS NULL OR s.form_id = $2)
                AND ($3::text IS NULL OR s.status = $3)
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
                status = $1,
                reviewed_by = $2,
                reviewed_at = NOW(),
                review_notes = $3,
                updated_at = NOW()
            WHERE id = $4 AND organization_id = $5
            RETURNING *
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

    // ========================================================================
    // Statistics
    // ========================================================================

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

    // ========================================================================
    // Download Tracking
    // ========================================================================

    /// Records a form download.
    pub async fn record_download<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO form_downloads (form_id, downloaded_by, ip_address, user_agent)
            VALUES ($1, $2, $3::inet, $4)
            "#,
        )
        .bind(form_id)
        .bind(user_id)
        .bind(&ip_address)
        .bind(&user_agent)
        .execute(executor)
        .await?;

        Ok(())
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

    // ========================================================================
    // Available Forms for Users
    // ========================================================================

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
                f.status,
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

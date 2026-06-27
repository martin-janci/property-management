//! Platform Migration & Data Import Repository (Epic 66).
//!
//! Handles database operations for import templates, import jobs,
//! import row errors, and migration exports.

use crate::models::migration::{
    CreateImportJob, CreateImportTemplate, CreateMigrationExport, ImportDataType, ImportJob,
    ImportJobStatus, ImportRowError, ImportTemplate, MigrationExport, MigrationExportStatus,
    UpdateImportTemplate,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for platform migration operations.
#[derive(Clone)]
pub struct MigrationRepository {
    pool: DbPool,
}

impl MigrationRepository {
    /// Create a new MigrationRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // IMPORT TEMPLATES
    // ========================================================================

    /// List templates for an organization, optionally including system templates.
    pub async fn list_templates<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data_type: Option<ImportDataType>,
        include_system: bool,
    ) -> Result<Vec<ImportTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        match (data_type, include_system) {
            (Some(dt), true) => {
                sqlx::query_as::<_, ImportTemplate>(
                    "SELECT * FROM import_templates WHERE (organization_id = $1 OR organization_id IS NULL) AND data_type = $2",
                )
                .bind(org_id)
                .bind(dt)
                .fetch_all(executor)
                .await
            }
            (Some(dt), false) => {
                sqlx::query_as::<_, ImportTemplate>(
                    "SELECT * FROM import_templates WHERE organization_id = $1 AND data_type = $2",
                )
                .bind(org_id)
                .bind(dt)
                .fetch_all(executor)
                .await
            }
            (None, true) => {
                sqlx::query_as::<_, ImportTemplate>(
                    "SELECT * FROM import_templates WHERE (organization_id = $1 OR organization_id IS NULL)",
                )
                .bind(org_id)
                .fetch_all(executor)
                .await
            }
            (None, false) => {
                sqlx::query_as::<_, ImportTemplate>(
                    "SELECT * FROM import_templates WHERE organization_id = $1",
                )
                .bind(org_id)
                .fetch_all(executor)
                .await
            }
        }
    }

    /// List system-provided templates.
    pub async fn list_system_templates<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<ImportTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportTemplate>(
            "SELECT * FROM import_templates WHERE is_system_template = true",
        )
        .fetch_all(executor)
        .await
    }

    /// Create a new template.
    pub async fn create_template<'e, E>(
        &self,
        executor: E,
        org_id: Option<Uuid>,
        name: String,
        description: Option<String>,
        data_type: ImportDataType,
        field_mappings: serde_json::Value,
        created_by: Option<Uuid>,
        is_system_template: bool,
    ) -> Result<ImportTemplate, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportTemplate>(
            r#"
            INSERT INTO import_templates (
                organization_id, name, description, data_type, field_mappings,
                is_system_template, version, is_active, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, 1, true, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(description)
        .bind(data_type)
        .bind(field_mappings)
        .bind(is_system_template)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Find a template by ID.
    pub async fn get_template<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
    ) -> Result<Option<ImportTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportTemplate>("SELECT * FROM import_templates WHERE id = $1")
            .bind(template_id)
            .fetch_optional(executor)
            .await
    }

    /// Update a template.
    pub async fn update_template<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
        name: Option<String>,
        description: Option<String>,
        field_mappings: Option<serde_json::Value>,
        is_active: Option<bool>,
    ) -> Result<ImportTemplate, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportTemplate>(
            r#"
            UPDATE import_templates
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                field_mappings = COALESCE($4, field_mappings),
                is_active = COALESCE($5, is_active),
                version = version + 1,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(template_id)
        .bind(name)
        .bind(description)
        .bind(field_mappings)
        .bind(is_active)
        .fetch_one(executor)
        .await
    }

    /// Delete a template.
    pub async fn delete_template<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
    ) -> Result<u64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM import_templates WHERE id = $1")
            .bind(template_id)
            .execute(executor)
            .await
            .map(|r| r.rows_affected())
    }

    /// Duplicate an existing template.
    pub async fn duplicate_template<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<ImportTemplate, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportTemplate>(
            r#"
            INSERT INTO import_templates (
                organization_id, name, description, data_type, field_mappings,
                is_system_template, version, is_active, created_by
            )
            SELECT $2, name || ' (Copy)', description, data_type, field_mappings,
                   false, 1, true, $3
            FROM import_templates
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(template_id)
        .bind(org_id)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    // ========================================================================
    // BULK DATA IMPORT JOBS
    // ========================================================================

    /// Create a new import job.
    pub async fn create_import_job<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        template_id: Uuid,
        status: ImportJobStatus,
        original_filename: String,
        file_path: String,
        file_size_bytes: i64,
        options: Option<serde_json::Value>,
        created_by: Uuid,
    ) -> Result<ImportJob, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportJob>(
            r#"
            INSERT INTO import_jobs (
                organization_id, template_id, status, original_filename, file_path,
                file_size_bytes, total_rows, processed_rows, successful_rows,
                failed_rows, skipped_rows, options, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, NULL, 0, 0, 0, 0, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(template_id)
        .bind(status)
        .bind(original_filename)
        .bind(file_path)
        .bind(file_size_bytes)
        .bind(options)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Find an import job by ID.
    pub async fn get_import_job<'e, E>(
        &self,
        executor: E,
        job_id: Uuid,
    ) -> Result<Option<ImportJob>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportJob>("SELECT * FROM import_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_optional(executor)
            .await
    }

    /// Update an import job.
    pub async fn update_import_job<'e, E>(
        &self,
        executor: E,
        job_id: Uuid,
        status: Option<ImportJobStatus>,
        total_rows: Option<i32>,
        processed_rows: Option<i32>,
        successful_rows: Option<i32>,
        failed_rows: Option<i32>,
        skipped_rows: Option<i32>,
        validation_errors: Option<serde_json::Value>,
        import_errors: Option<serde_json::Value>,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
    ) -> Result<ImportJob, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ImportJob>(
            r#"
            UPDATE import_jobs
            SET
                status = COALESCE($2, status),
                total_rows = COALESCE($3, total_rows),
                processed_rows = COALESCE($4, processed_rows),
                successful_rows = COALESCE($5, successful_rows),
                failed_rows = COALESCE($6, failed_rows),
                skipped_rows = COALESCE($7, skipped_rows),
                validation_errors = COALESCE($8, validation_errors),
                import_errors = COALESCE($9, import_errors),
                started_at = COALESCE($10, started_at),
                completed_at = COALESCE($11, completed_at),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(job_id)
        .bind(status)
        .bind(total_rows)
        .bind(processed_rows)
        .bind(successful_rows)
        .bind(failed_rows)
        .bind(skipped_rows)
        .bind(validation_errors)
        .bind(import_errors)
        .bind(started_at)
        .bind(completed_at)
        .fetch_one(executor)
        .await
    }

    /// List import jobs for an organization.
    pub async fn list_import_jobs<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        status: Option<ImportJobStatus>,
        data_type: Option<ImportDataType>,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<ImportJob>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = per_page.max(1) as i64;
        let offset = ((page.max(1) - 1) * per_page.max(1)) as i64;

        match (status, data_type) {
            (Some(s), Some(dt)) => {
                sqlx::query_as::<_, ImportJob>(
                    r#"
                    SELECT j.* FROM import_jobs j
                    JOIN import_templates t ON j.template_id = t.id
                    WHERE j.organization_id = $1 AND j.status = $2 AND t.data_type = $3
                    ORDER BY j.created_at DESC
                    LIMIT $4 OFFSET $5
                    "#,
                )
                .bind(org_id)
                .bind(s)
                .bind(dt)
                .bind(limit)
                .bind(offset)
                .fetch_all(executor)
                .await
            }
            (Some(s), None) => {
                sqlx::query_as::<_, ImportJob>(
                    r#"
                    SELECT * FROM import_jobs
                    WHERE organization_id = $1 AND status = $2
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(org_id)
                .bind(s)
                .bind(limit)
                .bind(offset)
                .fetch_all(executor)
                .await
            }
            (None, Some(dt)) => {
                sqlx::query_as::<_, ImportJob>(
                    r#"
                    SELECT j.* FROM import_jobs j
                    JOIN import_templates t ON j.template_id = t.id
                    WHERE j.organization_id = $1 AND t.data_type = $2
                    ORDER BY j.created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(org_id)
                .bind(dt)
                .bind(limit)
                .bind(offset)
                .fetch_all(executor)
                .await
            }
            (None, None) => {
                sqlx::query_as::<_, ImportJob>(
                    r#"
                    SELECT * FROM import_jobs
                    WHERE organization_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(org_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(executor)
                .await
            }
        }
    }

    /// List import jobs history.
    pub async fn list_import_jobs_history<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        status: Option<ImportJobStatus>,
        data_type: Option<ImportDataType>,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<ImportJobHistory>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = per_page.max(1) as i64;
        let offset = ((page.max(1) - 1) * per_page.max(1)) as i64;

        let mut query = String::from(
            r#"
            SELECT j.id, j.status, j.original_filename AS filename, t.data_type,
                   j.successful_rows AS records_imported, j.failed_rows AS records_failed,
                   COALESCE(u.name, 'Unknown') AS created_by_name,
                   j.created_at, j.completed_at
            FROM import_jobs j
            JOIN import_templates t ON j.template_id = t.id
            LEFT JOIN users u ON j.created_by = u.id
            WHERE j.organization_id = $1
            "#,
        );

        let mut param_idx = 2;
        let mut status_val = None;
        let mut dt_val = None;

        if let Some(s) = status {
            query.push_str(&format!(" AND j.status = ${}", param_idx));
            param_idx += 1;
            status_val = Some(s);
        }

        if let Some(dt) = data_type {
            query.push_str(&format!(" AND t.data_type = ${}", param_idx));
            param_idx += 1;
            dt_val = Some(dt);
        }

        query.push_str(&format!(
            " ORDER BY j.created_at DESC LIMIT ${} OFFSET ${}",
            param_idx,
            param_idx + 1
        ));

        let mut q = sqlx::query_as::<_, ImportJobHistory>(sqlx::AssertSqlSafe(&query)).bind(org_id);

        if let Some(s) = status_val {
            q = q.bind(s);
        }
        if let Some(dt) = dt_val {
            q = q.bind(dt);
        }

        q.bind(limit).bind(offset).fetch_all(executor).await
    }

    /// Count import jobs matching filters.
    pub async fn count_import_jobs<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        status: Option<ImportJobStatus>,
        data_type: Option<ImportDataType>,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count: (i64,) = match (status, data_type) {
            (Some(s), Some(dt)) => {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(j.id) FROM import_jobs j
                    JOIN import_templates t ON j.template_id = t.id
                    WHERE j.organization_id = $1 AND j.status = $2 AND t.data_type = $3
                    "#,
                )
                .bind(org_id)
                .bind(s)
                .bind(dt)
                .fetch_one(executor)
                .await?
            }
            (Some(s), None) => {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(id) FROM import_jobs
                    WHERE organization_id = $1 AND status = $2
                    "#,
                )
                .bind(org_id)
                .bind(s)
                .fetch_one(executor)
                .await?
            }
            (None, Some(dt)) => {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(j.id) FROM import_jobs j
                    JOIN import_templates t ON j.template_id = t.id
                    WHERE j.organization_id = $1 AND t.data_type = $2
                    "#,
                )
                .bind(org_id)
                .bind(dt)
                .fetch_one(executor)
                .await?
            }
            (None, None) => {
                sqlx::query_as("SELECT COUNT(id) FROM import_jobs WHERE organization_id = $1")
                    .bind(org_id)
                    .fetch_one(executor)
                    .await?
            }
        };
        Ok(count.0)
    }

    // ========================================================================
    // IMPORT ROW ERRORS
    // ========================================================================

    /// Create an import row error record.
    pub async fn create_import_row_error<'e, E>(
        &self,
        executor: E,
        job_id: Uuid,
        org_id: Uuid,
        error: &ImportRowError,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO import_row_errors (
                job_id, organization_id, row_number, "column", message, error_code, original_value
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(job_id)
        .bind(org_id)
        .bind(error.row_number)
        .bind(&error.column)
        .bind(&error.message)
        .bind(&error.error_code)
        .bind(&error.original_value)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// List errors for a specific import job.
    pub async fn list_import_job_errors<'e, E>(
        &self,
        executor: E,
        job_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<ImportRowError>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = per_page.max(1) as i64;
        let offset = ((page.max(1) - 1) * per_page.max(1)) as i64;

        sqlx::query_as::<_, ImportRowError>(
            r#"
            SELECT row_number, "column", message, error_code, original_value
            FROM import_row_errors
            WHERE job_id = $1
            ORDER BY row_number ASC, id ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(job_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Count errors for a specific import job.
    pub async fn count_import_job_errors<'e, E>(
        &self,
        executor: E,
        job_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(id) FROM import_row_errors WHERE job_id = $1")
                .bind(job_id)
                .fetch_one(executor)
                .await?;
        Ok(count.0)
    }

    // ========================================================================
    // MIGRATION EXPORTS
    // ========================================================================

    /// Create a migration export request.
    pub async fn create_migration_export<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        status: MigrationExportStatus,
        categories: serde_json::Value,
        privacy_options: serde_json::Value,
        output_format: String,
        expires_at: DateTime<Utc>,
        created_by: Uuid,
    ) -> Result<MigrationExport, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MigrationExport>(
            r#"
            INSERT INTO migration_exports (
                organization_id, status, categories, privacy_options, output_format,
                download_count, expires_at, created_by
            )
            VALUES ($1, $2, $3, $4, $5, 0, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(status)
        .bind(categories)
        .bind(privacy_options)
        .bind(output_format)
        .bind(expires_at)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Find a migration export by ID.
    pub async fn get_migration_export<'e, E>(
        &self,
        executor: E,
        export_id: Uuid,
    ) -> Result<Option<MigrationExport>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MigrationExport>("SELECT * FROM migration_exports WHERE id = $1")
            .bind(export_id)
            .fetch_optional(executor)
            .await
    }

    /// Find a migration export by download token.
    pub async fn get_migration_export_by_token<'e, E>(
        &self,
        executor: E,
        token: Uuid,
    ) -> Result<Option<MigrationExport>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MigrationExport>(
            "SELECT * FROM migration_exports WHERE download_token = $1",
        )
        .bind(token)
        .fetch_optional(executor)
        .await
    }

    /// Update a migration export record.
    pub async fn update_migration_export<'e, E>(
        &self,
        executor: E,
        export_id: Uuid,
        status: Option<MigrationExportStatus>,
        file_path: Option<String>,
        file_size_bytes: Option<i64>,
        file_hash: Option<String>,
        download_token: Option<Uuid>,
        download_count: Option<i32>,
        downloaded_at: Option<DateTime<Utc>>,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        error_message: Option<String>,
    ) -> Result<MigrationExport, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MigrationExport>(
            r#"
            UPDATE migration_exports
            SET
                status = COALESCE($2, status),
                file_path = COALESCE($3, file_path),
                file_size_bytes = COALESCE($4, file_size_bytes),
                file_hash = COALESCE($5, file_hash),
                download_token = COALESCE($6, download_token),
                download_count = COALESCE($7, download_count),
                downloaded_at = COALESCE($8, downloaded_at),
                started_at = COALESCE($9, started_at),
                completed_at = COALESCE($10, completed_at),
                error_message = COALESCE($11, error_message),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(export_id)
        .bind(status)
        .bind(file_path)
        .bind(file_size_bytes)
        .bind(file_hash)
        .bind(download_token)
        .bind(download_count)
        .bind(downloaded_at)
        .bind(started_at)
        .bind(completed_at)
        .bind(error_message)
        .fetch_one(executor)
        .await
    }

    /// List migration exports for an organization.
    pub async fn list_migration_exports<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<MigrationExport>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = per_page.max(1) as i64;
        let offset = ((page.max(1) - 1) * per_page.max(1)) as i64;

        sqlx::query_as::<_, MigrationExport>(
            r#"
            SELECT * FROM migration_exports
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Count migration exports for an organization.
    pub async fn count_migration_exports<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(id) FROM migration_exports WHERE organization_id = $1")
                .bind(org_id)
                .fetch_one(executor)
                .await?;
        Ok(count.0)
    }
}

//! Document template repository (Epic 7B: Story 7B.2 - Document Templates & Generation).

use crate::models::{
    CreateTemplate, DocumentTemplate, TemplateListQuery, TemplateSummary, TemplateWithDetails,
    UpdateTemplate,
};
use sqlx::{Error as SqlxError, PgPool, Row};
use uuid::Uuid;

/// Repository for document template operations.
#[derive(Debug, Clone)]
pub struct DocumentTemplateRepository {
    pool: PgPool,
}

impl DocumentTemplateRepository {
    /// Create a new template repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new template.
    ///
    /// `template_type` is a Postgres enum (`document_template_type`) but the
    /// model decodes it as `String`, so every query that returns it must cast
    /// it to text — `SELECT *` / `RETURNING *` fail to decode (PAP-133).
    pub async fn create(&self, data: CreateTemplate) -> Result<DocumentTemplate, SqlxError> {
        let placeholders = serde_json::to_value(&data.placeholders).unwrap();

        sqlx::query_as::<_, DocumentTemplate>(
            r#"
            INSERT INTO document_templates (
                organization_id, name, description, template_type,
                content, placeholders, created_by
            )
            VALUES ($1, $2, $3, $4::document_template_type, $5, $6, $7)
            RETURNING id, organization_id, name, description,
                template_type::text AS template_type, content, placeholders,
                usage_count, created_by, created_at, updated_at, deleted_at
            "#,
        )
        .bind(data.organization_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.template_type)
        .bind(&data.content)
        .bind(&placeholders)
        .bind(data.created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Find template by ID, scoped to the owning organization.
    ///
    /// Cross-tenant safety (PAP-63): this repository runs on the raw (non-RLS)
    /// pool, so the `document_templates` RLS policies never engage — app-layer
    /// scoping is the only tenant boundary. The `organization_id = $2`
    /// predicate ensures a caller in org B cannot read a template owned by
    /// org A by guessing its UUID.
    pub async fn find_by_id(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<DocumentTemplate>, SqlxError> {
        sqlx::query_as::<_, DocumentTemplate>(
            r#"
            SELECT id, organization_id, name, description,
                template_type::text AS template_type, content, placeholders,
                usage_count, created_by, created_at, updated_at, deleted_at
            FROM document_templates
            WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find template by ID with details, scoped to the owning organization.
    ///
    /// See [`find_by_id`](Self::find_by_id) for the cross-tenant rationale
    /// behind the `organization_id = $2` predicate (PAP-63).
    pub async fn find_by_id_with_details(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<TemplateWithDetails>, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT
                t.id, t.organization_id, t.name, t.description,
                t.template_type::text AS template_type, t.content,
                t.placeholders, t.usage_count, t.created_by, t.created_at,
                t.updated_at, t.deleted_at,
                u.name as created_by_name
            FROM document_templates t
            JOIN users u ON u.id = t.created_by
            WHERE t.id = $1 AND t.organization_id = $2 AND t.deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| TemplateWithDetails {
            template: DocumentTemplate {
                id: r.get("id"),
                organization_id: r.get("organization_id"),
                name: r.get("name"),
                description: r.get("description"),
                template_type: r.get("template_type"),
                content: r.get("content"),
                placeholders: r.get("placeholders"),
                usage_count: r.get("usage_count"),
                created_by: r.get("created_by"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                deleted_at: r.get("deleted_at"),
            },
            created_by_name: r.get("created_by_name"),
        }))
    }

    /// List templates for an organization.
    pub async fn list(
        &self,
        org_id: Uuid,
        query: TemplateListQuery,
    ) -> Result<Vec<TemplateSummary>, SqlxError> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, TemplateSummary>(
            r#"
            SELECT
                id, name, description,
                template_type::text AS template_type, usage_count,
                jsonb_array_length(placeholders)::bigint as placeholder_count,
                created_at
            FROM document_templates
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND ($2::text IS NULL OR template_type::text = $2)
              AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')
            ORDER BY name
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(&query.template_type)
        .bind(&query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count templates matching query.
    pub async fn count(&self, org_id: Uuid, query: TemplateListQuery) -> Result<i64, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM document_templates
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND ($2::text IS NULL OR template_type::text = $2)
              AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')
            "#,
        )
        .bind(org_id)
        .bind(&query.template_type)
        .bind(&query.search)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    /// Update a template, scoped to the owning organization.
    ///
    /// Cross-tenant safety (PAP-63): the `organization_id = $2` predicate is
    /// defense-in-depth on top of the handler's read gate — a mutation can
    /// never touch a row owned by another tenant even if a caller forgets to
    /// pre-check ownership.
    pub async fn update(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: UpdateTemplate,
    ) -> Result<DocumentTemplate, SqlxError> {
        let placeholders = data.placeholders.map(|p| serde_json::to_value(p).unwrap());

        sqlx::query_as::<_, DocumentTemplate>(
            r#"
            UPDATE document_templates
            SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                template_type = COALESCE($5::document_template_type, template_type),
                content = COALESCE($6, content),
                placeholders = COALESCE($7, placeholders),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
            RETURNING id, organization_id, name, description,
                template_type::text AS template_type, content, placeholders,
                usage_count, created_by, created_at, updated_at, deleted_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.template_type)
        .bind(&data.content)
        .bind(&placeholders)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a template (soft delete), scoped to the owning organization.
    ///
    /// Cross-tenant safety (PAP-63): the `organization_id = $2` predicate
    /// prevents a caller in org B from soft-deleting org A's template by
    /// guessing its UUID.
    pub async fn delete(&self, id: Uuid, org_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE document_templates
            SET deleted_at = NOW()
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Check if template name exists in organization.
    pub async fn name_exists(
        &self,
        org_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM document_templates
                WHERE organization_id = $1
                  AND name = $2
                  AND deleted_at IS NULL
                  AND ($3::uuid IS NULL OR id != $3)
            ) as exists
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("exists"))
    }

    /// Increment usage count for a template, scoped to the owning organization.
    ///
    /// Cross-tenant safety (PAP-63): the `organization_id = $2` predicate keeps
    /// the counter mutation within the caller's tenant.
    pub async fn increment_usage(&self, id: Uuid, org_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE document_templates
            SET usage_count = usage_count + 1
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

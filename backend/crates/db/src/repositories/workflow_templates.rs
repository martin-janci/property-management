//! Workflow templates repository (Epic 94, Story 94.4).
//!
//! Provides CRUD operations for workflow templates, including
//! search, import, and rating functionality.
//!
//! # RLS Integration (PAP-104 / PAP-80 / PAP-67)
//!
//! This repository previously held a raw `PgPool` and ran every query on it,
//! so a query could never set `app.current_org_id`. Migration `00179` (PAP-62)
//! put `FORCE ROW LEVEL SECURITY` + the canonical `get_current_org_id()` policy
//! on `workflows` and `workflow_actions`, which `import_template` writes to.
//! Under `FORCE` the api-server's owner connection is no longer exempt, so a
//! write issued on a connection WITHOUT RLS context set fails the policy
//! `WITH CHECK` (deny-all). To match the `work_order.rs` / `ai_chat.rs` /
//! `vendor.rs` precedent the repo now holds **no pool**: every method takes an
//! **executor whose connection already has RLS context set** — in handlers this
//! comes from the `RlsConnection` extractor via `&mut **rls.conn()`. There is no
//! way to issue a query that bypasses RLS.
//!
//! Single-statement methods take a generic `E: Executor`. Multi-statement
//! methods that must run on the SAME RLS-scoped connection
//! (`find_with_details`, `import_template`, `rate_template`,
//! `seed_builtin_templates`) take a `&mut PgConnection` and reborrow.
//!
//! > **Scaffold note (PAP-104):** the `workflow_template*` tables this repo's
//! > marketplace methods target are **not created by any migration** and the
//! > repository is **not constructed by any server** (the `/api/v1/ai/workflows`
//! > template handlers serve `get_builtin_templates()` instead). The conversion
//! > is retained so the repo is RLS-safe if/when the marketplace ships; see the
//! > PAP-104 issue thread for the dead-scaffold disposition question raised to
//! > the CTO.

use crate::models::{
    template_scope, CreateTemplateAction, CreateTemplateVariable, CreateWorkflowTemplate,
    ImportTemplateRequest, RateTemplateRequest, TemplateSearchQuery, UpdateWorkflowTemplate,
    WorkflowTemplate, WorkflowTemplateAction, WorkflowTemplateRating, WorkflowTemplateSummary,
    WorkflowTemplateVariable, WorkflowTemplateWithDetails,
};
use sqlx::{Connection, Executor, PgPool, Postgres};
use uuid::Uuid;

/// Repository for workflow template operations.
///
/// Deliberately a zero-sized type: it holds no pool so it cannot issue an
/// un-scoped (deny-all under `FORCE`) query. All queries run on a context-set
/// connection supplied by the handler's `RlsConnection`.
#[derive(Clone)]
pub struct WorkflowTemplateRepository;

impl WorkflowTemplateRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    /// Create a new workflow template.
    pub async fn create<'e, E>(
        &self,
        executor: E,
        data: CreateWorkflowTemplate,
    ) -> Result<WorkflowTemplate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO workflow_templates
                (organization_id, name, description, category, trigger_type, trigger_config,
                 conditions, scope, tags, icon, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.category)
        .bind(&data.trigger_type)
        .bind(sqlx::types::Json(data.trigger_config.unwrap_or_default()))
        .bind(sqlx::types::Json(data.conditions.unwrap_or_default()))
        .bind(
            data.scope
                .unwrap_or_else(|| template_scope::ORGANIZATION.to_string()),
        )
        .bind(data.tags.unwrap_or_default())
        .bind(&data.icon)
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Get template by ID.
    pub async fn find_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<WorkflowTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM workflow_templates WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Get template with full details (actions and variables).
    ///
    /// Takes a `&mut PgConnection` so the three reads (template + actions +
    /// variables) run on the SAME RLS-scoped connection.
    pub async fn find_with_details(
        &self,
        executor: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<WorkflowTemplateWithDetails>, sqlx::Error> {
        let template = self.find_by_id(&mut *executor, id).await?;
        match template {
            Some(t) => {
                let actions = self.list_actions(&mut *executor, id).await?;
                let variables = self.list_variables(&mut *executor, id).await?;
                Ok(Some(WorkflowTemplateWithDetails {
                    template: t,
                    actions,
                    variables,
                }))
            }
            None => Ok(None),
        }
    }

    /// Search templates with filters.
    pub async fn search<'e, E>(
        &self,
        executor: E,
        org_id: Option<Uuid>,
        query: TemplateSearchQuery,
    ) -> Result<Vec<WorkflowTemplateSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT
                t.id,
                t.name,
                t.description,
                t.category,
                t.trigger_type,
                t.scope,
                t.use_count,
                t.avg_rating,
                t.tags,
                t.icon,
                t.featured,
                COUNT(a.id) as action_count
            FROM workflow_templates t
            LEFT JOIN workflow_template_actions a ON a.template_id = t.id
            WHERE t.active = TRUE
                AND (
                    t.scope = 'global'
                    OR t.scope = 'platform'
                    OR (t.scope = 'organization' AND t.organization_id = $1)
                )
                AND ($2::text IS NULL OR t.category = $2)
                AND ($3::text IS NULL OR t.trigger_type = $3)
                AND ($4::text IS NULL OR t.name ILIKE '%' || $4 || '%' OR t.description ILIKE '%' || $4 || '%')
                AND ($5::boolean IS NULL OR t.featured = $5)
                AND ($6::text IS NULL OR t.scope = $6)
            GROUP BY t.id
            ORDER BY
                t.featured DESC,
                t.use_count DESC,
                t.avg_rating DESC NULLS LAST,
                t.name
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(&query.category)
        .bind(&query.trigger_type)
        .bind(&query.search)
        .bind(query.featured)
        .bind(&query.scope)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// List templates by category.
    pub async fn list_by_category<'e, E>(
        &self,
        executor: E,
        category: &str,
        org_id: Option<Uuid>,
    ) -> Result<Vec<WorkflowTemplateSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.search(
            executor,
            org_id,
            TemplateSearchQuery {
                category: Some(category.to_string()),
                ..Default::default()
            },
        )
        .await
    }

    /// List featured templates.
    pub async fn list_featured<'e, E>(
        &self,
        executor: E,
        org_id: Option<Uuid>,
    ) -> Result<Vec<WorkflowTemplateSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.search(
            executor,
            org_id,
            TemplateSearchQuery {
                featured: Some(true),
                limit: Some(10),
                ..Default::default()
            },
        )
        .await
    }

    /// Update a template.
    pub async fn update<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateWorkflowTemplate,
    ) -> Result<WorkflowTemplate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE workflow_templates SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                category = COALESCE($4, category),
                trigger_config = COALESCE($5, trigger_config),
                conditions = COALESCE($6, conditions),
                tags = COALESCE($7, tags),
                icon = COALESCE($8, icon),
                featured = COALESCE($9, featured),
                active = COALESCE($10, active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.category)
        .bind(data.trigger_config.map(sqlx::types::Json))
        .bind(data.conditions.map(sqlx::types::Json))
        .bind(&data.tags)
        .bind(&data.icon)
        .bind(data.featured)
        .bind(data.active)
        .fetch_one(executor)
        .await
    }

    /// Delete a template.
    pub async fn delete<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM workflow_templates WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // --- Actions ---

    /// Add an action to a template.
    pub async fn add_action<'e, E>(
        &self,
        executor: E,
        data: CreateTemplateAction,
    ) -> Result<WorkflowTemplateAction, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO workflow_template_actions
                (template_id, action_order, action_type, action_config, description,
                 on_failure, retry_count, retry_delay_seconds)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.template_id)
        .bind(data.action_order)
        .bind(&data.action_type)
        .bind(sqlx::types::Json(&data.action_config))
        .bind(&data.description)
        .bind(data.on_failure.unwrap_or_else(|| "stop".to_string()))
        .bind(data.retry_count.unwrap_or(3))
        .bind(data.retry_delay_seconds.unwrap_or(60))
        .fetch_one(executor)
        .await
    }

    /// List actions for a template.
    pub async fn list_actions<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
    ) -> Result<Vec<WorkflowTemplateAction>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM workflow_template_actions WHERE template_id = $1 ORDER BY action_order",
        )
        .bind(template_id)
        .fetch_all(executor)
        .await
    }

    /// Delete an action.
    pub async fn delete_action<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM workflow_template_actions WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // --- Variables ---

    /// Add a variable to a template.
    pub async fn add_variable<'e, E>(
        &self,
        executor: E,
        data: CreateTemplateVariable,
    ) -> Result<WorkflowTemplateVariable, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO workflow_template_variables
                (template_id, name, label, description, variable_type,
                 default_value, required, options, validation_pattern)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(data.template_id)
        .bind(&data.name)
        .bind(&data.label)
        .bind(&data.description)
        .bind(&data.variable_type)
        .bind(&data.default_value)
        .bind(data.required.unwrap_or(false))
        .bind(data.options.map(sqlx::types::Json))
        .bind(&data.validation_pattern)
        .fetch_one(executor)
        .await
    }

    /// List variables for a template.
    pub async fn list_variables<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
    ) -> Result<Vec<WorkflowTemplateVariable>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM workflow_template_variables WHERE template_id = $1 ORDER BY name",
        )
        .bind(template_id)
        .fetch_all(executor)
        .await
    }

    /// Delete a variable.
    pub async fn delete_variable<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM workflow_template_variables WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // --- Import ---

    /// Import a template as a new workflow.
    /// Returns the new workflow ID.
    ///
    /// Takes a `&mut PgConnection` so the template read, the workflow + action
    /// inserts (into the `FORCE`-RLS `workflows` / `workflow_actions` tables),
    /// and the use-count bump all run on the SAME RLS-scoped connection inside
    /// one transaction. `org_id` and `user_id` must originate from the verified
    /// request principal, never from client input.
    pub async fn import_template(
        &self,
        executor: &mut sqlx::PgConnection,
        org_id: Uuid,
        user_id: Uuid,
        request: ImportTemplateRequest,
    ) -> Result<Uuid, sqlx::Error> {
        // Get the template with details (same RLS-scoped connection).
        let template_details = self
            .find_with_details(&mut *executor, request.template_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let template = &template_details.template;

        // Start transaction for all write operations (on the context-set conn,
        // so the org/user GUCs stay in scope for every policy check).
        let mut tx = executor.begin().await?;

        // Create the workflow
        let workflow_name = request.name.unwrap_or_else(|| template.name.clone());

        let workflow: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO workflows
                (organization_id, name, description, trigger_type, trigger_config,
                 conditions, enabled, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(&workflow_name)
        .bind(&template.description)
        .bind(&template.trigger_type)
        .bind(&template.trigger_config)
        .bind(&template.conditions)
        .bind(request.enabled.unwrap_or(false))
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        let workflow_id = workflow.0;

        // Copy actions with variable substitution
        for action in &template_details.actions {
            let mut config = action.action_config.0.clone();

            // Substitute variables in the config
            if let Some(vars) = request.variables.as_object() {
                substitute_variables(&mut config, vars);
            }

            sqlx::query(
                r#"
                INSERT INTO workflow_actions
                    (workflow_id, action_order, action_type, action_config,
                     on_failure, retry_count, retry_delay_seconds)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(workflow_id)
            .bind(action.action_order)
            .bind(&action.action_type)
            .bind(sqlx::types::Json(&config))
            .bind(&action.on_failure)
            .bind(action.retry_count)
            .bind(action.retry_delay_seconds)
            .execute(&mut *tx)
            .await?;
        }

        // Increment use count
        sqlx::query("UPDATE workflow_templates SET use_count = use_count + 1 WHERE id = $1")
            .bind(request.template_id)
            .execute(&mut *tx)
            .await?;

        // Commit transaction
        tx.commit().await?;

        Ok(workflow_id)
    }

    // --- Ratings ---

    /// Rate a template.
    ///
    /// Takes a `&mut PgConnection` so the rating upsert and the average-rating
    /// recompute run on the SAME RLS-scoped connection. `org_id` and `user_id`
    /// must originate from the verified request principal.
    pub async fn rate_template(
        &self,
        executor: &mut sqlx::PgConnection,
        template_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        request: RateTemplateRequest,
    ) -> Result<WorkflowTemplateRating, sqlx::Error> {
        // Upsert the rating
        let rating: WorkflowTemplateRating = sqlx::query_as(
            r#"
            INSERT INTO workflow_template_ratings (template_id, organization_id, user_id, rating, review)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (template_id, organization_id, user_id) DO UPDATE SET
                rating = EXCLUDED.rating,
                review = EXCLUDED.review,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(template_id)
        .bind(org_id)
        .bind(user_id)
        .bind(request.rating.clamp(1, 5))
        .bind(&request.review)
        .fetch_one(&mut *executor)
        .await?;

        // Update average rating
        sqlx::query(
            r#"
            UPDATE workflow_templates
            SET avg_rating = (
                SELECT AVG(rating)::real
                FROM workflow_template_ratings
                WHERE template_id = $1
            )
            WHERE id = $1
            "#,
        )
        .bind(template_id)
        .execute(&mut *executor)
        .await?;

        Ok(rating)
    }

    /// Get ratings for a template.
    pub async fn list_ratings<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkflowTemplateRating>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM workflow_template_ratings
            WHERE template_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(template_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Seed built-in templates.
    ///
    /// Takes a `&mut PgConnection` so the existence check, template insert, and
    /// per-action inserts all run on the SAME RLS-scoped connection.
    pub async fn seed_builtin_templates(
        &self,
        executor: &mut sqlx::PgConnection,
    ) -> Result<usize, sqlx::Error> {
        let templates = crate::models::get_builtin_templates();
        let mut count = 0;

        for (template_data, actions) in templates {
            // Check if template already exists by name
            let existing: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM workflow_templates WHERE name = $1 AND scope = 'global'",
            )
            .bind(&template_data.name)
            .fetch_optional(&mut *executor)
            .await?;

            if existing.is_some() {
                continue;
            }

            // Create template
            let template = self.create(&mut *executor, template_data).await?;

            // Add actions
            for mut action in actions {
                action.template_id = template.id;
                self.add_action(&mut *executor, action).await?;
            }

            count += 1;
        }

        Ok(count)
    }
}

/// Substitute variables in a JSON value.
fn substitute_variables(
    value: &mut serde_json::Value,
    vars: &serde_json::Map<String, serde_json::Value>,
) {
    match value {
        serde_json::Value::String(s) => {
            for (key, val) in vars {
                let placeholder = format!("{{{{{}}}}}", key);
                if let Some(replacement) = val.as_str() {
                    *s = s.replace(&placeholder, replacement);
                }
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                substitute_variables(v, vars);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                substitute_variables(v, vars);
            }
        }
        _ => {}
    }
}

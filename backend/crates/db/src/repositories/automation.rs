//! Workflow Automation repository (Epic 38).
//!
//! Repository for automation rules, templates, and execution logs.

use crate::models::automation::*;
use crate::DbPool;
use chrono::Utc;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for workflow automation operations.
#[derive(Clone)]
pub struct AutomationRepository {
    pool: DbPool,
}

impl AutomationRepository {
    /// Create a new AutomationRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Automation Rules (Story 38.1)
    // ========================================================================

    /// Create an automation rule.
    pub async fn create_rule(
        &self,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateAutomationRule,
    ) -> Result<WorkflowAutomationRule, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationRule>(
            r#"
            INSERT INTO workflow_automation_rules (
                organization_id, name, description, trigger_type, trigger_config,
                conditions, actions, is_active, created_by
            )
            -- trigger_type is a Postgres ENUM (workflow_automation_trigger); the String
            -- bind must be cast on encode, and cast back to text on decode.
            VALUES ($1, $2, $3, $4::workflow_automation_trigger, $5, $6, $7, COALESCE($8, true), $9)
            RETURNING
                id, organization_id, name, description, trigger_type::text AS trigger_type,
                trigger_config, conditions, actions, is_active, last_run_at, next_run_at,
                run_count, error_count, last_error, created_by, created_at, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.trigger_type)
        .bind(&data.trigger_config)
        .bind(&data.conditions)
        .bind(&data.actions)
        .bind(data.is_active)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get automation rule by ID (no tenant scoping — internal use only).
    ///
    /// Handlers MUST use [`Self::find_rule_for_org`] to enforce tenant
    /// isolation. This unscoped variant is retained for background workers
    /// (scheduled execution, executor lookups) where the `organization_id`
    /// is derived from the persisted row itself rather than from a caller.
    pub async fn get_rule(&self, id: Uuid) -> Result<Option<WorkflowAutomationRule>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationRule>(
            r#"
            SELECT
                id, organization_id, name, description, trigger_type::text AS trigger_type,
                trigger_config, conditions, actions, is_active, last_run_at, next_run_at,
                run_count, error_count, last_error, created_by, created_at, updated_at
            FROM workflow_automation_rules WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get automation rule by ID, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the rule does not exist OR belongs to a
    /// different organization — the caller MUST surface this as a 404 so
    /// cross-tenant existence is not leaked via the status code.
    /// Mirrors `WorkflowRepository::find_by_id_for_org`.
    pub async fn find_rule_for_org(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<WorkflowAutomationRule>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationRule>(
            r#"
            SELECT
                id, organization_id, name, description, trigger_type::text AS trigger_type,
                trigger_config, conditions, actions, is_active, last_run_at, next_run_at,
                run_count, error_count, last_error, created_by, created_at, updated_at
            FROM workflow_automation_rules WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List automation rules for an organization.
    pub async fn list_rules(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<WorkflowAutomationRule>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationRule>(
            r#"
            SELECT
                id, organization_id, name, description, trigger_type::text AS trigger_type,
                trigger_config, conditions, actions, is_active, last_run_at, next_run_at,
                run_count, error_count, last_error, created_by, created_at, updated_at
            FROM workflow_automation_rules WHERE organization_id = $1 ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update automation rule, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the rule does not exist OR belongs to a
    /// different organization. The caller MUST surface this as 404 to avoid
    /// leaking cross-tenant existence.
    pub async fn update_rule(
        &self,
        id: Uuid,
        organization_id: Uuid,
        data: UpdateAutomationRule,
    ) -> Result<Option<WorkflowAutomationRule>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationRule>(
            r#"
            UPDATE workflow_automation_rules SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                trigger_config = COALESCE($5, trigger_config),
                conditions = COALESCE($6, conditions),
                actions = COALESCE($7, actions),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING
                id, organization_id, name, description, trigger_type::text AS trigger_type,
                trigger_config, conditions, actions, is_active, last_run_at, next_run_at,
                run_count, error_count, last_error, created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.trigger_config)
        .bind(&data.conditions)
        .bind(&data.actions)
        .bind(data.is_active)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete automation rule, scoped to the caller's organization.
    pub async fn delete_rule(&self, id: Uuid, organization_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "DELETE FROM workflow_automation_rules WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(organization_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Toggle rule active status, scoped to the caller's organization.
    ///
    /// Returns `Ok(false)` when no row was updated (rule missing OR belongs
    /// to a different organization). Caller surfaces this as 404.
    pub async fn toggle_rule(
        &self,
        id: Uuid,
        organization_id: Uuid,
        is_active: bool,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "UPDATE workflow_automation_rules SET is_active = $3, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(organization_id)
        .bind(is_active)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Automation Logs
    // ========================================================================

    /// Log rule execution.
    pub async fn log_execution(
        &self,
        rule_id: Uuid,
        trigger_data: Option<serde_json::Value>,
    ) -> Result<WorkflowAutomationLog, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationLog>(
            r#"
            INSERT INTO workflow_automation_logs (rule_id, trigger_data, status)
            VALUES ($1, $2, 'running')
            RETURNING *
            "#,
        )
        .bind(rule_id)
        .bind(&trigger_data)
        .fetch_one(&self.pool)
        .await
    }

    /// Complete log execution.
    pub async fn complete_execution(
        &self,
        log_id: Uuid,
        status: &str,
        actions_executed: serde_json::Value,
        error_message: Option<String>,
    ) -> Result<(), SqlxError> {
        let started_at: chrono::DateTime<Utc> =
            sqlx::query_scalar("SELECT started_at FROM workflow_automation_logs WHERE id = $1")
                .bind(log_id)
                .fetch_one(&self.pool)
                .await?;

        let duration_ms = (Utc::now() - started_at).num_milliseconds() as i32;

        sqlx::query(
            r#"
            UPDATE workflow_automation_logs SET
                status = $2,
                actions_executed = $3,
                error_message = $4,
                completed_at = NOW(),
                duration_ms = $5
            WHERE id = $1
            "#,
        )
        .bind(log_id)
        .bind(status)
        .bind(&actions_executed)
        .bind(&error_message)
        .bind(duration_ms)
        .execute(&self.pool)
        .await?;

        // Update rule stats
        if status == "success" {
            sqlx::query(
                r#"
                UPDATE workflow_automation_rules SET
                    run_count = run_count + 1,
                    last_run_at = NOW()
                WHERE id = (SELECT rule_id FROM workflow_automation_logs WHERE id = $1)
                "#,
            )
            .bind(log_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE workflow_automation_rules SET
                    run_count = run_count + 1,
                    error_count = error_count + 1,
                    last_run_at = NOW(),
                    last_error = $2
                WHERE id = (SELECT rule_id FROM workflow_automation_logs WHERE id = $1)
                "#,
            )
            .bind(log_id)
            .bind(&error_message)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get execution logs for a rule (no tenant scoping — internal use only).
    ///
    /// Handlers MUST use [`Self::get_rule_logs_for_org`].
    pub async fn get_rule_logs(
        &self,
        rule_id: Uuid,
        limit: i32,
    ) -> Result<Vec<WorkflowAutomationLog>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationLog>(
            r#"
            SELECT * FROM workflow_automation_logs
            WHERE rule_id = $1
            ORDER BY started_at DESC
            LIMIT $2
            "#,
        )
        .bind(rule_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Get execution logs for a rule, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the rule does not exist OR belongs to a
    /// different organization. We do a pre-check on the rule (rather than
    /// JOIN-and-return-empty) so the caller can distinguish "no logs yet"
    /// from "wrong tenant" and emit a 404 in the latter case — otherwise
    /// every cross-tenant probe would return 200 with `[]` and silently
    /// confirm the row exists in *some* org.
    pub async fn get_rule_logs_for_org(
        &self,
        rule_id: Uuid,
        organization_id: Uuid,
        limit: i32,
    ) -> Result<Option<Vec<WorkflowAutomationLog>>, SqlxError> {
        let owner_org: Option<Uuid> = sqlx::query_scalar(
            "SELECT organization_id FROM workflow_automation_rules WHERE id = $1",
        )
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await?;

        match owner_org {
            Some(owner) if owner == organization_id => {
                let logs = sqlx::query_as::<_, WorkflowAutomationLog>(
                    r#"
                    SELECT * FROM workflow_automation_logs
                    WHERE rule_id = $1
                    ORDER BY started_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(rule_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;
                Ok(Some(logs))
            }
            _ => Ok(None),
        }
    }

    // ========================================================================
    // Automation Templates
    // ========================================================================

    /// List all templates.
    pub async fn list_templates(&self) -> Result<Vec<WorkflowAutomationTemplate>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationTemplate>(
            r#"
            SELECT
                id, name, description, category, trigger_type::text AS trigger_type,
                trigger_config_template, actions_template, is_system, created_at
            FROM workflow_automation_templates ORDER BY category, name
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Get template by ID.
    pub async fn get_template(
        &self,
        id: Uuid,
    ) -> Result<Option<WorkflowAutomationTemplate>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationTemplate>(
            r#"
            SELECT
                id, name, description, category, trigger_type::text AS trigger_type,
                trigger_config_template, actions_template, is_system, created_at
            FROM workflow_automation_templates WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create rule from template.
    pub async fn create_from_template(
        &self,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateRuleFromTemplate,
    ) -> Result<WorkflowAutomationRule, SqlxError> {
        let template = self
            .get_template(data.template_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Deep merge overrides with template
        let trigger_config = if let Some(overrides) = data.trigger_config_overrides {
            deep_merge_json(template.trigger_config_template.clone(), overrides)
        } else {
            template.trigger_config_template
        };

        let actions = if let Some(overrides) = data.actions_overrides {
            overrides
        } else {
            template.actions_template
        };

        self.create_rule(
            organization_id,
            created_by,
            CreateAutomationRule {
                name: data.name,
                description: data.description.or(template.description),
                trigger_type: template.trigger_type,
                trigger_config,
                conditions: None,
                actions,
                is_active: Some(true),
            },
        )
        .await
    }

    // ========================================================================
    // Scheduled Execution
    // ========================================================================

    /// Get rules due for scheduled execution.
    pub async fn get_due_rules(&self) -> Result<Vec<WorkflowAutomationRule>, SqlxError> {
        sqlx::query_as::<_, WorkflowAutomationRule>(
            r#"
            SELECT
                id, organization_id, name, description, trigger_type::text AS trigger_type,
                trigger_config, conditions, actions, is_active, last_run_at, next_run_at,
                run_count, error_count, last_error, created_by, created_at, updated_at
            FROM workflow_automation_rules
            WHERE is_active = true
              AND trigger_type = 'schedule'
              AND (next_run_at IS NULL OR next_run_at <= NOW())
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Update next run time for a scheduled rule.
    pub async fn update_next_run(
        &self,
        id: Uuid,
        next_run_at: chrono::DateTime<Utc>,
    ) -> Result<(), SqlxError> {
        sqlx::query("UPDATE workflow_automation_rules SET next_run_at = $2 WHERE id = $1")
            .bind(id)
            .bind(next_run_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Deep merge two JSON values, with overrides taking precedence.
/// For objects, recursively merges nested values.
/// For other types, override replaces base entirely.
fn deep_merge_json(base: serde_json::Value, overrides: serde_json::Value) -> serde_json::Value {
    match (base, overrides) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(override_map)) => {
            for (key, override_value) in override_map {
                base_map.insert(
                    key.clone(),
                    if let Some(base_value) = base_map.get(&key) {
                        deep_merge_json(base_value.clone(), override_value)
                    } else {
                        override_value
                    },
                );
            }
            serde_json::Value::Object(base_map)
        }
        // For non-objects, override replaces entirely
        (_, overrides) => overrides,
    }
}

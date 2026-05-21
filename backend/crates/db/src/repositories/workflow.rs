//! Workflow automation repository (Epic 13, Story 13.6 & 13.7).

use crate::models::{
    CreateWorkflow, CreateWorkflowAction, ExecutionQuery, TriggerWorkflow, UpdateWorkflow,
    Workflow, WorkflowAction, WorkflowExecution, WorkflowExecutionStep, WorkflowQuery,
    WorkflowSchedule, WorkflowSummary,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Hard ceiling on `retry_count` per action (H4).
///
/// Exponential backoff with attempt=N grows as `2^(N-1) * delay`. Even with
/// a 1-second base delay, attempt=31 would compute to ~68 years before the
/// runtime clamp kicks in. 10 retries is more than enough for any sane
/// integration and keeps `2^(retries-1)` within `u64` without surprise.
pub const MAX_RETRY_COUNT: i32 = 10;

/// Hard ceiling on `retry_delay_seconds` per action (H4).
///
/// One hour. The executor caps the *effective* exponential delay at the
/// same value; clamping the base here just prevents the multiplier from
/// blowing past `u64` arithmetic in the executor.
pub const MAX_RETRY_DELAY_SECONDS: i32 = 3600;

/// Repository for workflow operations.
#[derive(Clone)]
pub struct WorkflowRepository {
    pool: PgPool,
}

impl WorkflowRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new workflow.
    /// Create a workflow.
    ///
    /// `organization_id` and `created_by` are passed explicitly by the caller
    /// and must originate from the verified request principal, never from
    /// client input in `data`.
    pub async fn create(
        &self,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateWorkflow,
    ) -> Result<Workflow, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO workflows
                (organization_id, name, description, trigger_type, trigger_config, conditions, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.name)
        .bind(data.description)
        .bind(data.trigger_type)
        .bind(sqlx::types::Json(data.trigger_config.unwrap_or_default()))
        .bind(sqlx::types::Json(data.conditions.unwrap_or_default()))
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get workflow by ID (no tenant scoping — internal use by the executor only).
    ///
    /// Handlers must use [`Self::find_by_id_for_org`] to enforce tenant
    /// isolation. This unscoped variant is retained for the workflow executor,
    /// which loads workflows during background execution where the
    /// `organization_id` is derived from the persisted row itself.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Workflow>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get workflow by ID, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the workflow does not exist OR belongs to a
    /// different organization — the caller MUST surface this as a 404 to
    /// avoid disclosing cross-tenant existence.
    pub async fn find_by_id_for_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Workflow>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM workflows WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List workflows with filters.
    pub async fn list(
        &self,
        org_id: Uuid,
        query: WorkflowQuery,
    ) -> Result<Vec<WorkflowSummary>, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT
                w.id,
                w.name,
                w.trigger_type,
                w.enabled,
                w.trigger_count,
                w.last_triggered_at,
                COUNT(a.id) as action_count
            FROM workflows w
            LEFT JOIN workflow_actions a ON a.workflow_id = w.id
            WHERE w.organization_id = $1
                AND ($2::text IS NULL OR w.trigger_type = $2)
                AND ($3::boolean IS NULL OR w.enabled = $3)
                AND ($4::text IS NULL OR w.name ILIKE '%' || $4 || '%')
            GROUP BY w.id
            ORDER BY w.name
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(org_id)
        .bind(query.trigger_type)
        .bind(query.enabled)
        .bind(query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a workflow, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the workflow does not exist OR belongs to a
    /// different organization. The caller MUST surface this as 404.
    pub async fn update(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: UpdateWorkflow,
    ) -> Result<Option<Workflow>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE workflows SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                trigger_type = COALESCE($5, trigger_type),
                trigger_config = COALESCE($6, trigger_config),
                conditions = COALESCE($7, conditions),
                enabled = COALESCE($8, enabled),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(data.name)
        .bind(data.description)
        .bind(data.trigger_type)
        .bind(data.trigger_config.map(sqlx::types::Json))
        .bind(data.conditions.map(sqlx::types::Json))
        .bind(data.enabled)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a workflow, scoped to the caller's organization.
    pub async fn delete(&self, id: Uuid, org_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM workflows WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Add an action to a workflow, scoped to the caller's organization.
    ///
    /// Verifies that `data.workflow_id` belongs to `org_id` BEFORE inserting,
    /// so a cross-tenant caller cannot attach actions to another org's
    /// workflow. Returns `Ok(None)` on tenant mismatch — the caller MUST
    /// surface this as 404.
    ///
    /// Also clamps `retry_count` and `retry_delay_seconds` at insertion time
    /// — see [`MAX_RETRY_COUNT`] / [`MAX_RETRY_DELAY_SECONDS`] (H4 defense).
    pub async fn add_action(
        &self,
        org_id: Uuid,
        data: CreateWorkflowAction,
    ) -> Result<Option<WorkflowAction>, sqlx::Error> {
        // Tenant check: the workflow must belong to the caller's org.
        let belongs: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM workflows WHERE id = $1 AND organization_id = $2")
                .bind(data.workflow_id)
                .bind(org_id)
                .fetch_optional(&self.pool)
                .await?;
        if belongs.is_none() {
            return Ok(None);
        }

        // H4 (insertion-time clamp): cap retry parameters to safe bounds so
        // a malicious or buggy client can't queue a retry that sleeps for
        // years. The executor applies the same clamps at runtime as
        // defense-in-depth.
        let retry_count = data.retry_count.unwrap_or(3).clamp(0, MAX_RETRY_COUNT);
        let retry_delay_seconds = data
            .retry_delay_seconds
            .unwrap_or(60)
            .clamp(0, MAX_RETRY_DELAY_SECONDS);

        let action = sqlx::query_as(
            r#"
            INSERT INTO workflow_actions
                (workflow_id, action_order, action_type, action_config, on_failure, retry_count, retry_delay_seconds)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(data.workflow_id)
        .bind(data.action_order)
        .bind(data.action_type)
        .bind(sqlx::types::Json(data.action_config))
        .bind(data.on_failure.unwrap_or_else(|| "stop".to_string()))
        .bind(retry_count)
        .bind(retry_delay_seconds)
        .fetch_one(&self.pool)
        .await?;
        Ok(Some(action))
    }

    /// Get actions for a workflow, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the workflow does not exist OR belongs to a
    /// different organization. The caller MUST surface this as 404.
    pub async fn list_actions(
        &self,
        workflow_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Vec<WorkflowAction>>, sqlx::Error> {
        // Tenant check first — without this, a cross-tenant caller could
        // probe action shapes by trying random workflow IDs.
        let belongs: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM workflows WHERE id = $1 AND organization_id = $2")
                .bind(workflow_id)
                .bind(org_id)
                .fetch_optional(&self.pool)
                .await?;
        if belongs.is_none() {
            return Ok(None);
        }

        let actions = sqlx::query_as(
            "SELECT * FROM workflow_actions WHERE workflow_id = $1 ORDER BY action_order",
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(actions))
    }

    /// Delete an action, scoped to the caller's organization.
    ///
    /// The action is identified by its own ID. We JOIN through workflows to
    /// confirm the parent workflow belongs to `org_id`; otherwise the delete
    /// is a no-op and returns `false`, which the caller surfaces as 404.
    pub async fn delete_action(&self, id: Uuid, org_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM workflow_actions a
            USING workflows w
            WHERE a.id = $1
              AND a.workflow_id = w.id
              AND w.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Create an execution record.
    pub async fn create_execution(
        &self,
        data: TriggerWorkflow,
    ) -> Result<WorkflowExecution, sqlx::Error> {
        let execution: WorkflowExecution = sqlx::query_as(
            r#"
            INSERT INTO workflow_executions (workflow_id, trigger_event, context, status, started_at)
            VALUES ($1, $2, $3, 'running', NOW())
            RETURNING *
            "#,
        )
        .bind(data.workflow_id)
        .bind(sqlx::types::Json(data.trigger_event))
        .bind(sqlx::types::Json(data.context.unwrap_or_default()))
        .fetch_one(&self.pool)
        .await?;

        // Update workflow trigger stats
        sqlx::query(
            "UPDATE workflows SET last_triggered_at = NOW(), trigger_count = trigger_count + 1 WHERE id = $1",
        )
        .bind(data.workflow_id)
        .execute(&self.pool)
        .await?;

        Ok(execution)
    }

    /// Get execution by ID (no tenant scoping — internal use only).
    ///
    /// HTTP handlers must use [`Self::find_execution_by_id_for_org`].
    pub async fn find_execution_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<WorkflowExecution>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM workflow_executions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get execution by ID, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the execution does not exist OR belongs to a
    /// workflow in a different organization. The caller MUST surface this
    /// as 404.
    pub async fn find_execution_by_id_for_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<WorkflowExecution>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT e.* FROM workflow_executions e
            JOIN workflows w ON w.id = e.workflow_id
            WHERE e.id = $1 AND w.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List executions with filters.
    pub async fn list_executions(
        &self,
        org_id: Uuid,
        query: ExecutionQuery,
    ) -> Result<Vec<WorkflowExecution>, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT e.* FROM workflow_executions e
            JOIN workflows w ON w.id = e.workflow_id
            WHERE w.organization_id = $1
                AND ($2::uuid IS NULL OR e.workflow_id = $2)
                AND ($3::text IS NULL OR e.status = $3)
                AND ($4::timestamptz IS NULL OR e.created_at >= $4)
                AND ($5::timestamptz IS NULL OR e.created_at <= $5)
            ORDER BY e.created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(query.workflow_id)
        .bind(query.status)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update execution status.
    pub async fn update_execution_status(
        &self,
        id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<WorkflowExecution, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE workflow_executions SET
                status = $2,
                error_message = $3,
                completed_at = CASE WHEN $2 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error_message)
        .fetch_one(&self.pool)
        .await
    }

    /// Create an execution step record.
    pub async fn create_execution_step(
        &self,
        execution_id: Uuid,
        action_id: Uuid,
    ) -> Result<WorkflowExecutionStep, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO workflow_execution_steps (execution_id, action_id, status, started_at)
            VALUES ($1, $2, 'running', NOW())
            RETURNING *
            "#,
        )
        .bind(execution_id)
        .bind(action_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Update execution step.
    pub async fn update_execution_step(
        &self,
        id: Uuid,
        status: &str,
        output: serde_json::Value,
        error_message: Option<&str>,
        duration_ms: Option<i32>,
    ) -> Result<WorkflowExecutionStep, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE workflow_execution_steps SET
                status = $2,
                output = $3,
                error_message = $4,
                duration_ms = $5,
                completed_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(sqlx::types::Json(output))
        .bind(error_message)
        .bind(duration_ms)
        .fetch_one(&self.pool)
        .await
    }

    /// Get execution steps (no tenant scoping — internal use only).
    ///
    /// HTTP handlers must use [`Self::list_execution_steps_for_org`].
    pub async fn list_execution_steps(
        &self,
        execution_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionStep>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT s.* FROM workflow_execution_steps s
            JOIN workflow_actions a ON a.id = s.action_id
            WHERE s.execution_id = $1
            ORDER BY a.action_order
            "#,
        )
        .bind(execution_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get execution steps, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the execution does not exist OR belongs to a
    /// workflow in a different organization. The caller MUST surface this
    /// as 404 — disclosing presence/absence cross-tenant is itself a leak.
    pub async fn list_execution_steps_for_org(
        &self,
        execution_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Vec<WorkflowExecutionStep>>, sqlx::Error> {
        // Tenant check via the parent workflow.
        let belongs: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT e.id FROM workflow_executions e
            JOIN workflows w ON w.id = e.workflow_id
            WHERE e.id = $1 AND w.organization_id = $2
            "#,
        )
        .bind(execution_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;
        if belongs.is_none() {
            return Ok(None);
        }

        let steps = sqlx::query_as(
            r#"
            SELECT s.* FROM workflow_execution_steps s
            JOIN workflow_actions a ON a.id = s.action_id
            WHERE s.execution_id = $1
            ORDER BY a.action_order
            "#,
        )
        .bind(execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(steps))
    }

    /// Get workflows by trigger type (for event matching).
    pub async fn list_by_trigger_type(
        &self,
        org_id: Uuid,
        trigger_type: &str,
    ) -> Result<Vec<Workflow>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM workflows WHERE organization_id = $1 AND trigger_type = $2 AND enabled = TRUE",
        )
        .bind(org_id)
        .bind(trigger_type)
        .fetch_all(&self.pool)
        .await
    }

    /// Create or update workflow schedule.
    pub async fn upsert_schedule(
        &self,
        workflow_id: Uuid,
        cron_expression: &str,
        timezone: &str,
        next_run_at: DateTime<Utc>,
    ) -> Result<WorkflowSchedule, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO workflow_schedules (workflow_id, cron_expression, timezone, next_run_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (workflow_id) DO UPDATE SET
                cron_expression = EXCLUDED.cron_expression,
                timezone = EXCLUDED.timezone,
                next_run_at = EXCLUDED.next_run_at,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(workflow_id)
        .bind(cron_expression)
        .bind(timezone)
        .bind(next_run_at)
        .fetch_one(&self.pool)
        .await
    }

    /// Get due scheduled workflows.
    pub async fn list_due_schedules(&self) -> Result<Vec<WorkflowSchedule>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM workflow_schedules WHERE enabled = TRUE AND next_run_at <= NOW()",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Update schedule after execution.
    pub async fn update_schedule_after_run(
        &self,
        id: Uuid,
        next_run_at: DateTime<Utc>,
    ) -> Result<WorkflowSchedule, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE workflow_schedules
            SET last_run_at = NOW(), next_run_at = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(next_run_at)
        .fetch_one(&self.pool)
        .await
    }
}

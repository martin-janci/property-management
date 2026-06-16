//! Workflow automation repository (Epic 13, Story 13.6 & 13.7).
//!
//! # RLS Integration (PAP-77 / PAP-67 / PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the workflow tables (`workflows`,
//! `workflow_actions`, `workflow_executions`, `workflow_execution_steps`,
//! `workflow_schedules`). Under `FORCE` the api-server's owner connection is
//! no longer exempt, so a query issued on a connection without
//! `app.current_org_id` set collapses to deny-all.
//!
//! Every method therefore takes an **executor whose connection already has
//! RLS context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`; in the background
//! workflow executor from `RlsPool::acquire_with_rls` with the org derived
//! from the already-loaded workflow row. The repository holds **no pool**, so
//! there is no way to issue a query that bypasses RLS. This mirrors the
//! `board_meetings.rs` / `vendor.rs` / `budget.rs` precedent.
//!
//! Cross-tenant safety (PAP-133 defense-in-depth): RLS is the primary
//! boundary, but by-id queries stay org-keyed — a connection whose role
//! bypasses RLS (superuser pools, BYPASSRLS) must still never resolve another
//! tenant's row. `workflows` carries its own `organization_id`; the child
//! tables (`workflow_actions`, `workflow_executions`,
//! `workflow_execution_steps`) are keyed through `workflows.organization_id`
//! via JOIN / EXISTS. The previously-unscoped internal variants
//! (`find_by_id`, `find_execution_by_id`, `list_execution_steps`) were
//! removed: the executor now receives the loaded `Workflow` row from its
//! caller, so no unscoped by-id read remains.

use crate::models::{
    CreateWorkflow, CreateWorkflowAction, ExecutionQuery, TriggerWorkflow, UpdateWorkflow,
    Workflow, WorkflowAction, WorkflowExecution, WorkflowExecutionStep, WorkflowQuery,
    WorkflowSchedule, WorkflowSummary,
};
use chrono::{DateTime, Utc};
use sqlx::{Executor, PgConnection, PgPool, Postgres};
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
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`)
/// query.
#[derive(Debug, Clone)]
pub struct WorkflowRepository;

impl WorkflowRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set
    /// connection supplied by the caller).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    /// Create a workflow.
    ///
    /// `organization_id` and `created_by` are passed explicitly by the caller
    /// and must originate from the verified request principal, never from
    /// client input in `data`.
    pub async fn create<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateWorkflow,
    ) -> Result<Workflow, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
    }

    /// Get workflow by ID, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the workflow does not exist OR belongs to a
    /// different organization — the caller MUST surface this as a 404 to
    /// avoid disclosing cross-tenant existence.
    pub async fn find_by_id_for_org<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Workflow>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM workflows WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .fetch_optional(executor)
            .await
    }

    /// List workflows with filters.
    pub async fn list<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: WorkflowQuery,
    ) -> Result<Vec<WorkflowSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_all(executor)
        .await
    }

    /// Update a workflow, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the workflow does not exist OR belongs to a
    /// different organization. The caller MUST surface this as 404.
    pub async fn update<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateWorkflow,
    ) -> Result<Option<Workflow>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_optional(executor)
        .await
    }

    /// Delete a workflow, scoped to the caller's organization.
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM workflows WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .execute(executor)
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
        conn: &mut PgConnection,
        org_id: Uuid,
        data: CreateWorkflowAction,
    ) -> Result<Option<WorkflowAction>, sqlx::Error> {
        // Tenant check: the workflow must belong to the caller's org.
        let belongs: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM workflows WHERE id = $1 AND organization_id = $2")
                .bind(data.workflow_id)
                .bind(org_id)
                .fetch_optional(&mut *conn)
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
        .fetch_one(&mut *conn)
        .await?;
        Ok(Some(action))
    }

    /// Get actions for a workflow, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the workflow does not exist OR belongs to a
    /// different organization. The caller MUST surface this as 404.
    pub async fn list_actions(
        &self,
        conn: &mut PgConnection,
        workflow_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Vec<WorkflowAction>>, sqlx::Error> {
        // Tenant check first — without this, a cross-tenant caller could
        // probe action shapes by trying random workflow IDs.
        let belongs: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM workflows WHERE id = $1 AND organization_id = $2")
                .bind(workflow_id)
                .bind(org_id)
                .fetch_optional(&mut *conn)
                .await?;
        if belongs.is_none() {
            return Ok(None);
        }

        let actions = sqlx::query_as(
            "SELECT * FROM workflow_actions WHERE workflow_id = $1 ORDER BY action_order",
        )
        .bind(workflow_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(Some(actions))
    }

    /// Delete an action, scoped to the caller's organization.
    ///
    /// The action is identified by its own ID. We JOIN through workflows to
    /// confirm the parent workflow belongs to `org_id`; otherwise the delete
    /// is a no-op and returns `false`, which the caller surfaces as 404.
    pub async fn delete_action<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Create an execution record.
    ///
    /// The caller must have verified that `data.workflow_id` belongs to the
    /// org whose RLS context is set on `conn` — under `FORCE` RLS the INSERT
    /// fails the policy otherwise (the policy keys `workflow_executions`
    /// through the parent workflow's org).
    pub async fn create_execution(
        &self,
        conn: &mut PgConnection,
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
        .fetch_one(&mut *conn)
        .await?;

        // Update workflow trigger stats
        sqlx::query(
            "UPDATE workflows SET last_triggered_at = NOW(), trigger_count = trigger_count + 1 WHERE id = $1",
        )
        .bind(data.workflow_id)
        .execute(&mut *conn)
        .await?;

        Ok(execution)
    }

    /// Get execution by ID, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the execution does not exist OR belongs to a
    /// workflow in a different organization. The caller MUST surface this
    /// as 404.
    pub async fn find_execution_by_id_for_org<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<WorkflowExecution>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT e.* FROM workflow_executions e
            JOIN workflows w ON w.id = e.workflow_id
            WHERE e.id = $1 AND w.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List executions with filters.
    pub async fn list_executions<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ExecutionQuery,
    ) -> Result<Vec<WorkflowExecution>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_all(executor)
        .await
    }

    /// Update execution status.
    ///
    /// Internal use by the workflow executor only — the execution is
    /// identified by its own ID; the RLS context on the connection (set from
    /// the owning workflow's org) is what scopes the write.
    pub async fn update_execution_status<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<WorkflowExecution, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
    }

    /// Create an execution step record (internal use by the executor).
    pub async fn create_execution_step<'e, E>(
        &self,
        executor: E,
        execution_id: Uuid,
        action_id: Uuid,
    ) -> Result<WorkflowExecutionStep, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO workflow_execution_steps (execution_id, action_id, status, started_at)
            VALUES ($1, $2, 'running', NOW())
            RETURNING *
            "#,
        )
        .bind(execution_id)
        .bind(action_id)
        .fetch_one(executor)
        .await
    }

    /// Update execution step (internal use by the executor).
    pub async fn update_execution_step<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        output: serde_json::Value,
        error_message: Option<&str>,
        duration_ms: Option<i32>,
    ) -> Result<WorkflowExecutionStep, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
    }

    /// Get execution steps, scoped to the caller's organization.
    ///
    /// Returns `Ok(None)` when the execution does not exist OR belongs to a
    /// workflow in a different organization. The caller MUST surface this
    /// as 404 — disclosing presence/absence cross-tenant is itself a leak.
    pub async fn list_execution_steps_for_org(
        &self,
        conn: &mut PgConnection,
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
        .fetch_optional(&mut *conn)
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
        .fetch_all(&mut *conn)
        .await?;
        Ok(Some(steps))
    }

    /// Get workflows by trigger type (for event matching).
    pub async fn list_by_trigger_type<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        trigger_type: &str,
    ) -> Result<Vec<Workflow>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM workflows WHERE organization_id = $1 AND trigger_type = $2 AND enabled = TRUE",
        )
        .bind(org_id)
        .bind(trigger_type)
        .fetch_all(executor)
        .await
    }

    /// Create or update workflow schedule.
    pub async fn upsert_schedule<'e, E>(
        &self,
        executor: E,
        workflow_id: Uuid,
        cron_expression: &str,
        timezone: &str,
        next_run_at: DateTime<Utc>,
    ) -> Result<WorkflowSchedule, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
    }

    /// Get due scheduled workflows.
    ///
    /// Under `FORCE` RLS this returns the due schedules of the org whose
    /// context is set on the connection — a future cross-org scheduler must
    /// iterate orgs with per-org contexts, not bypass RLS.
    pub async fn list_due_schedules<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<WorkflowSchedule>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM workflow_schedules WHERE enabled = TRUE AND next_run_at <= NOW()",
        )
        .fetch_all(executor)
        .await
    }

    /// Update schedule after execution.
    pub async fn update_schedule_after_run<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        next_run_at: DateTime<Utc>,
    ) -> Result<WorkflowSchedule, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
    }
}

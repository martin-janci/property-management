//! Work orders and maintenance scheduling repository (Epic 20).
//!
//! # RLS Integration (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `work_orders`, `work_order_updates`,
//! `maintenance_schedules`, and `schedule_executions`. Under `FORCE` the
//! api-server's owner connection is no longer exempt, so a query issued on a
//! connection without `app.current_org_id` set collapses to deny-all (own-org
//! reads return empty, writes fail).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `budget.rs` / `document.rs` precedent.

use crate::models::{
    AddWorkOrderUpdate, CreateMaintenanceSchedule, CreateWorkOrder, MaintenanceCostSummary,
    MaintenanceSchedule, ScheduleExecution, ScheduleQuery, ServiceHistoryEntry, UpcomingSchedule,
    UpdateMaintenanceSchedule, UpdateWorkOrder, WorkOrder, WorkOrderQuery, WorkOrderStatistics,
    WorkOrderUpdate, WorkOrderWithDetails,
};
use chrono::NaiveDate;
use sqlx::{Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

/// Repository for work order and maintenance schedule operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct WorkOrderRepository;

impl WorkOrderRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ========================================================================
    // Work Orders (Story 20.2)
    // ========================================================================

    /// Create a new work order.
    pub async fn create_work_order<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateWorkOrder,
    ) -> Result<WorkOrder, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO work_orders
                (organization_id, building_id, equipment_id, fault_id, title, description,
                 priority, work_type, assigned_to, vendor_id, scheduled_date, due_date,
                 estimated_cost, tags, metadata, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.equipment_id)
        .bind(data.fault_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.priority.as_deref().unwrap_or("medium"))
        .bind(data.work_type.as_deref().unwrap_or("corrective"))
        .bind(data.assigned_to)
        .bind(data.vendor_id)
        .bind(data.scheduled_date)
        .bind(data.due_date)
        .bind(data.estimated_cost)
        .bind(data.tags.unwrap_or_default())
        .bind(sqlx::types::Json(data.metadata.unwrap_or_default()))
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Create work order from a fault (Story 20.2 - fault-triggered work orders).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_from_fault<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        fault_id: Uuid,
        building_id: Uuid,
        title: &str,
        description: &str,
        priority: &str,
    ) -> Result<WorkOrder, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO work_orders
                (organization_id, building_id, fault_id, title, description,
                 priority, work_type, source, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, 'corrective', 'fault', $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(fault_id)
        .bind(title)
        .bind(description)
        .bind(priority)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Create work order from a maintenance schedule.
    pub async fn create_from_schedule<'e, E>(
        &self,
        executor: E,
        schedule: &MaintenanceSchedule,
        due_date: NaiveDate,
    ) -> Result<WorkOrder, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO work_orders
                (organization_id, building_id, equipment_id, title, description,
                 priority, work_type, assigned_to, vendor_id, due_date,
                 estimated_cost, source, schedule_id, created_by)
            VALUES ($1, $2, $3, $4, $5, 'medium', $6, $7, $8, $9, $10, 'schedule', $11, $12)
            RETURNING *
            "#,
        )
        .bind(schedule.organization_id)
        .bind(schedule.building_id)
        .bind(schedule.equipment_id)
        .bind(&schedule.name)
        .bind(&schedule.description)
        .bind(&schedule.work_type)
        .bind(schedule.default_assignee)
        .bind(schedule.default_vendor_id)
        .bind(due_date)
        .bind(schedule.estimated_cost)
        .bind(schedule.id)
        .bind(schedule.created_by)
        .fetch_one(executor)
        .await
    }

    /// Find work order by ID.
    ///
    /// RLS scopes the result to the connection's org context, so a work order
    /// owned by another organization is invisible (returns `None`).
    pub async fn find_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<WorkOrder>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM work_orders WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List work orders with filters.
    pub async fn list<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: WorkOrderQuery,
    ) -> Result<Vec<WorkOrder>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT * FROM work_orders
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::uuid IS NULL OR equipment_id = $3)
                AND ($4::uuid IS NULL OR fault_id = $4)
                AND ($5::uuid IS NULL OR assigned_to = $5)
                AND ($6::uuid IS NULL OR vendor_id = $6)
                AND ($7::text IS NULL OR status = $7)
                AND ($8::text IS NULL OR priority = $8)
                AND ($9::text IS NULL OR work_type = $9)
                AND ($10::text IS NULL OR source = $10)
                AND ($11::date IS NULL OR due_date <= $11)
                AND ($12::date IS NULL OR due_date >= $12)
            ORDER BY
                CASE priority
                    WHEN 'urgent' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                due_date NULLS LAST,
                created_at DESC
            LIMIT $13 OFFSET $14
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.equipment_id)
        .bind(query.fault_id)
        .bind(query.assigned_to)
        .bind(query.vendor_id)
        .bind(query.status)
        .bind(query.priority)
        .bind(query.work_type)
        .bind(query.source)
        .bind(query.due_before)
        .bind(query.due_after)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// List work orders with details (building name, equipment name, assignee name).
    pub async fn list_with_details<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: WorkOrderQuery,
    ) -> Result<Vec<WorkOrderWithDetails>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT
                wo.id,
                wo.title,
                wo.description,
                wo.priority,
                wo.work_type,
                wo.status,
                b.name as building_name,
                e.name as equipment_name,
                u.email as assigned_to_name,
                wo.due_date,
                wo.created_at
            FROM work_orders wo
            JOIN buildings b ON b.id = wo.building_id
            LEFT JOIN equipment e ON e.id = wo.equipment_id
            LEFT JOIN users u ON u.id = wo.assigned_to
            WHERE wo.organization_id = $1
                AND ($2::uuid IS NULL OR wo.building_id = $2)
                AND ($3::text IS NULL OR wo.status = $3)
                AND ($4::text IS NULL OR wo.priority = $4)
            ORDER BY
                CASE wo.priority
                    WHEN 'urgent' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                wo.due_date NULLS LAST
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.status)
        .bind(query.priority)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update work order.
    ///
    /// Multi-statement (read-modify + status-change log), so it takes a
    /// connection and reborrows it for each query.
    pub async fn update(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        user_id: Uuid,
        data: UpdateWorkOrder,
    ) -> Result<WorkOrder, sqlx::Error> {
        // Get old work order for tracking changes
        let old = self
            .find_by_id(&mut *conn, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // Update the work order
        let updated: WorkOrder = sqlx::query_as(
            r#"
            UPDATE work_orders SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                priority = COALESCE($4, priority),
                work_type = COALESCE($5, work_type),
                assigned_to = COALESCE($6, assigned_to),
                vendor_id = COALESCE($7, vendor_id),
                scheduled_date = COALESCE($8, scheduled_date),
                due_date = COALESCE($9, due_date),
                estimated_cost = COALESCE($10, estimated_cost),
                actual_cost = COALESCE($11, actual_cost),
                status = COALESCE($12, status),
                resolution_notes = COALESCE($13, resolution_notes),
                tags = COALESCE($14, tags),
                metadata = COALESCE($15, metadata),
                started_at = CASE
                    WHEN $12 = 'in_progress' AND started_at IS NULL THEN NOW()
                    ELSE started_at
                END,
                completed_at = CASE
                    WHEN $12 IN ('completed', 'cancelled') AND completed_at IS NULL THEN NOW()
                    ELSE completed_at
                END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.title)
        .bind(data.description)
        .bind(data.priority)
        .bind(data.work_type)
        .bind(data.assigned_to)
        .bind(data.vendor_id)
        .bind(data.scheduled_date)
        .bind(data.due_date)
        .bind(data.estimated_cost)
        .bind(data.actual_cost)
        .bind(&data.status)
        .bind(data.resolution_notes)
        .bind(data.tags)
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(&mut *conn)
        .await?;

        // Track status change
        if let Some(new_status) = &data.status {
            if &old.status != new_status {
                self.add_update(
                    &mut *conn,
                    id,
                    user_id,
                    "status_change",
                    &format!("Status changed from {} to {}", old.status, new_status),
                    Some(&old.status),
                    Some(new_status),
                )
                .await?;
            }
        }

        Ok(updated)
    }

    /// Delete work order.
    pub async fn delete<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM work_orders WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Assign work order to user or vendor.
    pub async fn assign(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        user_id: Uuid,
        assigned_to: Option<Uuid>,
        vendor_id: Option<Uuid>,
    ) -> Result<WorkOrder, sqlx::Error> {
        let updated: WorkOrder = sqlx::query_as(
            r#"
            UPDATE work_orders SET
                assigned_to = $2,
                vendor_id = $3,
                status = CASE
                    WHEN status = 'open' THEN 'assigned'
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(assigned_to)
        .bind(vendor_id)
        .fetch_one(&mut *conn)
        .await?;

        // Track assignment
        let assignee = if let Some(uid) = assigned_to {
            format!("user:{}", uid)
        } else if let Some(vid) = vendor_id {
            format!("vendor:{}", vid)
        } else {
            "unassigned".to_string()
        };

        self.add_update(
            &mut *conn,
            id,
            user_id,
            "assignment",
            &format!("Assigned to {}", assignee),
            None,
            Some(&assignee),
        )
        .await?;

        Ok(updated)
    }

    /// Start work on a work order.
    pub async fn start_work(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkOrder, sqlx::Error> {
        let updated: WorkOrder = sqlx::query_as(
            r#"
            UPDATE work_orders SET
                status = 'in_progress',
                started_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;

        self.add_update(
            &mut *conn,
            id,
            user_id,
            "status_change",
            "Work started",
            Some("assigned"),
            Some("in_progress"),
        )
        .await?;

        Ok(updated)
    }

    /// Complete work order.
    pub async fn complete(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        user_id: Uuid,
        actual_cost: Option<rust_decimal::Decimal>,
        resolution_notes: Option<&str>,
    ) -> Result<WorkOrder, sqlx::Error> {
        let updated: WorkOrder = sqlx::query_as(
            r#"
            UPDATE work_orders SET
                status = 'completed',
                actual_cost = COALESCE($2, actual_cost),
                resolution_notes = COALESCE($3, resolution_notes),
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(actual_cost)
        .bind(resolution_notes)
        .fetch_one(&mut *conn)
        .await?;

        self.add_update(
            &mut *conn,
            id,
            user_id,
            "status_change",
            "Work completed",
            Some("in_progress"),
            Some("completed"),
        )
        .await?;

        Ok(updated)
    }

    /// Put work order on hold.
    pub async fn put_on_hold(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        user_id: Uuid,
        reason: &str,
    ) -> Result<WorkOrder, sqlx::Error> {
        let old = self
            .find_by_id(&mut *conn, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let updated: WorkOrder = sqlx::query_as(
            r#"
            UPDATE work_orders SET
                status = 'on_hold',
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;

        self.add_update(
            &mut *conn,
            id,
            user_id,
            "status_change",
            &format!("Put on hold: {}", reason),
            Some(&old.status),
            Some("on_hold"),
        )
        .await?;

        Ok(updated)
    }

    /// Add comment/update to work order.
    pub async fn add_update<'e, E>(
        &self,
        executor: E,
        work_order_id: Uuid,
        user_id: Uuid,
        update_type: &str,
        content: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) -> Result<WorkOrderUpdate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO work_order_updates
                (work_order_id, user_id, update_type, content, old_value, new_value)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(work_order_id)
        .bind(user_id)
        .bind(update_type)
        .bind(content)
        .bind(old_value)
        .bind(new_value)
        .fetch_one(executor)
        .await
    }

    /// Add user comment to work order.
    pub async fn add_comment<'e, E>(
        &self,
        executor: E,
        work_order_id: Uuid,
        user_id: Uuid,
        data: AddWorkOrderUpdate,
    ) -> Result<WorkOrderUpdate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.add_update(
            executor,
            work_order_id,
            user_id,
            data.update_type.as_deref().unwrap_or("comment"),
            &data.content,
            None,
            None,
        )
        .await
    }

    /// List updates/comments for a work order.
    pub async fn list_updates<'e, E>(
        &self,
        executor: E,
        work_order_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<WorkOrderUpdate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM work_order_updates
            WHERE work_order_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(work_order_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Get work order statistics.
    pub async fn get_statistics<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<WorkOrderStatistics, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::bigint as total,
                COUNT(*) FILTER (WHERE status = 'open')::bigint as open,
                COUNT(*) FILTER (WHERE status = 'in_progress')::bigint as in_progress,
                COUNT(*) FILTER (WHERE status = 'completed')::bigint as completed,
                COUNT(*) FILTER (WHERE due_date < CURRENT_DATE AND status NOT IN ('completed', 'cancelled'))::bigint as overdue,
                AVG(EXTRACT(EPOCH FROM (completed_at - created_at)) / 86400) FILTER (WHERE completed_at IS NOT NULL) as avg_completion_days,
                SUM(actual_cost) as total_cost
            FROM work_orders
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// Get overdue work orders.
    pub async fn list_overdue<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        limit: i32,
    ) -> Result<Vec<WorkOrder>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM work_orders
            WHERE organization_id = $1
                AND due_date < CURRENT_DATE
                AND status NOT IN ('completed', 'cancelled')
            ORDER BY due_date, priority
            LIMIT $2
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // Maintenance Schedules (Story 20.3)
    // ========================================================================

    /// Create a maintenance schedule.
    pub async fn create_schedule<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateMaintenanceSchedule,
    ) -> Result<MaintenanceSchedule, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Calculate first next_due_date based on start_date
        let next_due = data.start_date;

        sqlx::query_as(
            r#"
            INSERT INTO maintenance_schedules
                (organization_id, building_id, equipment_id, name, description, work_type,
                 frequency, day_of_week, day_of_month, month_of_year,
                 default_assignee, default_vendor_id, start_date, end_date, next_due_date,
                 auto_create_work_order, advance_days, estimated_duration_hours, estimated_cost,
                 checklist, metadata, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.equipment_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.work_type.as_deref().unwrap_or("preventive"))
        .bind(&data.frequency)
        .bind(data.day_of_week)
        .bind(data.day_of_month)
        .bind(data.month_of_year)
        .bind(data.default_assignee)
        .bind(data.default_vendor_id)
        .bind(data.start_date)
        .bind(data.end_date)
        .bind(next_due)
        .bind(data.auto_create_work_order.unwrap_or(true))
        .bind(data.advance_days.unwrap_or(7))
        .bind(data.estimated_duration_hours)
        .bind(data.estimated_cost)
        .bind(sqlx::types::Json(
            data.checklist
                .map(|c| serde_json::json!(c))
                .unwrap_or_default(),
        ))
        .bind(sqlx::types::Json(data.metadata.unwrap_or_default()))
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Find schedule by ID.
    pub async fn find_schedule_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<MaintenanceSchedule>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM maintenance_schedules WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List schedules with filters.
    pub async fn list_schedules<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ScheduleQuery,
    ) -> Result<Vec<MaintenanceSchedule>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT * FROM maintenance_schedules
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::uuid IS NULL OR equipment_id = $3)
                AND ($4::text IS NULL OR frequency = $4)
                AND ($5::boolean IS NULL OR is_active = $5)
                AND ($6::date IS NULL OR next_due_date <= $6)
            ORDER BY next_due_date
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.equipment_id)
        .bind(query.frequency)
        .bind(query.is_active)
        .bind(query.due_before)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update schedule.
    pub async fn update_schedule<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateMaintenanceSchedule,
    ) -> Result<MaintenanceSchedule, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE maintenance_schedules SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                work_type = COALESCE($4, work_type),
                frequency = COALESCE($5, frequency),
                day_of_week = COALESCE($6, day_of_week),
                day_of_month = COALESCE($7, day_of_month),
                month_of_year = COALESCE($8, month_of_year),
                default_assignee = COALESCE($9, default_assignee),
                default_vendor_id = COALESCE($10, default_vendor_id),
                end_date = COALESCE($11, end_date),
                auto_create_work_order = COALESCE($12, auto_create_work_order),
                advance_days = COALESCE($13, advance_days),
                estimated_duration_hours = COALESCE($14, estimated_duration_hours),
                estimated_cost = COALESCE($15, estimated_cost),
                is_active = COALESCE($16, is_active),
                checklist = COALESCE($17, checklist),
                metadata = COALESCE($18, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.name)
        .bind(data.description)
        .bind(data.work_type)
        .bind(data.frequency)
        .bind(data.day_of_week)
        .bind(data.day_of_month)
        .bind(data.month_of_year)
        .bind(data.default_assignee)
        .bind(data.default_vendor_id)
        .bind(data.end_date)
        .bind(data.auto_create_work_order)
        .bind(data.advance_days)
        .bind(data.estimated_duration_hours)
        .bind(data.estimated_cost)
        .bind(data.is_active)
        .bind(
            data.checklist
                .map(|c| sqlx::types::Json(serde_json::json!(c))),
        )
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Delete schedule.
    pub async fn delete_schedule<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM maintenance_schedules WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Activate/deactivate schedule.
    pub async fn set_schedule_active<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        is_active: bool,
    ) -> Result<MaintenanceSchedule, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE maintenance_schedules SET
                is_active = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(is_active)
        .fetch_one(executor)
        .await
    }

    /// Get schedules due for execution.
    pub async fn get_due_schedules<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<MaintenanceSchedule>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM maintenance_schedules
            WHERE organization_id = $1
                AND is_active = TRUE
                AND auto_create_work_order = TRUE
                AND next_due_date <= CURRENT_DATE + ($2 || ' days')::interval
                AND (end_date IS NULL OR end_date >= CURRENT_DATE)
            ORDER BY next_due_date
            "#,
        )
        .bind(org_id)
        .bind(days_ahead)
        .fetch_all(executor)
        .await
    }

    /// Update schedule after work order created.
    ///
    /// Multi-statement (records an execution row, then advances the schedule),
    /// so it takes a connection.
    pub async fn mark_schedule_executed(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        work_order_id: Uuid,
    ) -> Result<MaintenanceSchedule, sqlx::Error> {
        // Record execution
        sqlx::query(
            r#"
            INSERT INTO schedule_executions
                (schedule_id, work_order_id, due_date, executed_at, status)
            SELECT $1, $2, next_due_date, NOW(), 'created'
            FROM maintenance_schedules WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(work_order_id)
        .execute(&mut *conn)
        .await?;

        // Update schedule next_due_date
        sqlx::query_as(
            r#"
            UPDATE maintenance_schedules SET
                last_run_date = CURRENT_DATE,
                next_due_date = calculate_next_schedule_date(
                    frequency, next_due_date, day_of_week, day_of_month, month_of_year
                ),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&mut *conn)
        .await
    }

    /// Skip a scheduled maintenance.
    pub async fn skip_schedule_execution(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        reason: &str,
    ) -> Result<MaintenanceSchedule, sqlx::Error> {
        // Record skipped execution
        sqlx::query(
            r#"
            INSERT INTO schedule_executions
                (schedule_id, due_date, status, skipped_reason)
            SELECT $1, next_due_date, 'skipped', $2
            FROM maintenance_schedules WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(reason)
        .execute(&mut *conn)
        .await?;

        // Update next_due_date
        sqlx::query_as(
            r#"
            UPDATE maintenance_schedules SET
                next_due_date = calculate_next_schedule_date(
                    frequency, next_due_date, day_of_week, day_of_month, month_of_year
                ),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&mut *conn)
        .await
    }

    /// Get upcoming schedules.
    pub async fn get_upcoming_schedules<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        days_ahead: i32,
        limit: i32,
    ) -> Result<Vec<UpcomingSchedule>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                ms.id,
                ms.name,
                ms.frequency,
                ms.next_due_date,
                e.name as equipment_name,
                b.name as building_name,
                (ms.next_due_date - CURRENT_DATE)::int as days_until_due
            FROM maintenance_schedules ms
            LEFT JOIN equipment e ON e.id = ms.equipment_id
            LEFT JOIN buildings b ON b.id = ms.building_id
            WHERE ms.organization_id = $1
                AND ms.is_active = TRUE
                AND ms.next_due_date <= CURRENT_DATE + ($2 || ' days')::interval
            ORDER BY ms.next_due_date
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(days_ahead)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Get execution history for a schedule.
    pub async fn list_executions<'e, E>(
        &self,
        executor: E,
        schedule_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ScheduleExecution>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM schedule_executions
            WHERE schedule_id = $1
            ORDER BY due_date DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(schedule_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // Service History & Reporting (Story 20.4)
    // ========================================================================

    /// Get service history for equipment.
    ///
    /// RLS on `work_orders` scopes this to the connection's org, so the query is
    /// implicitly tenant-bounded even though it is keyed by `equipment_id`.
    pub async fn get_service_history<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ServiceHistoryEntry>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                wo.id,
                wo.id as work_order_id,
                wo.title,
                wo.work_type,
                wo.completed_at,
                wo.actual_cost,
                wo.resolution_notes,
                e.name as equipment_name
            FROM work_orders wo
            LEFT JOIN equipment e ON e.id = wo.equipment_id
            WHERE wo.equipment_id = $1
                AND wo.status = 'completed'
            ORDER BY wo.completed_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(equipment_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Get maintenance cost summary by type.
    pub async fn get_cost_summary<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<MaintenanceCostSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                work_type,
                COUNT(*)::bigint as work_order_count,
                SUM(actual_cost) as total_cost,
                AVG(actual_cost) as avg_cost
            FROM work_orders
            WHERE organization_id = $1
                AND status = 'completed'
                AND completed_at >= $2
                AND completed_at < $3
            GROUP BY work_type
            ORDER BY total_cost DESC NULLS LAST
            "#,
        )
        .bind(org_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(executor)
        .await
    }

    /// Get service history for building.
    ///
    /// RLS on `work_orders` scopes this to the connection's org.
    pub async fn get_building_service_history<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ServiceHistoryEntry>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                wo.id,
                wo.id as work_order_id,
                wo.title,
                wo.work_type,
                wo.completed_at,
                wo.actual_cost,
                wo.resolution_notes,
                e.name as equipment_name
            FROM work_orders wo
            LEFT JOIN equipment e ON e.id = wo.equipment_id
            WHERE wo.building_id = $1
                AND wo.status = 'completed'
            ORDER BY wo.completed_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(building_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // Background Job Helpers
    // ========================================================================

    /// Process due schedules and create work orders.
    ///
    /// Multi-statement loop, so it takes a connection and reborrows it per
    /// schedule.
    pub async fn process_due_schedules(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<Vec<WorkOrder>, sqlx::Error> {
        let schedules = self.get_due_schedules(&mut *conn, org_id, 0).await?;
        let mut created_orders = Vec::new();

        for schedule in schedules {
            // Create work order from schedule
            let work_order = self
                .create_from_schedule(&mut *conn, &schedule, schedule.next_due_date)
                .await?;

            // Mark schedule as executed
            self.mark_schedule_executed(&mut *conn, schedule.id, work_order.id)
                .await?;

            created_orders.push(work_order);
        }

        Ok(created_orders)
    }
}

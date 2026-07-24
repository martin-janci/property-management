//! Equipment and predictive maintenance repository (Epic 13, Story 13.3).
//!
//! # RLS Integration (PAP-67 / PAP-71)
//!
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `equipment`, `equipment_maintenance`, and
//! `maintenance_predictions`. The api-server connects as the table OWNER, which
//! `FORCE` binds, so a query issued on a connection without `app.current_org_id`
//! set collapses to deny-all (own-org reads return empty, writes fail the policy
//! `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` / `document.rs` precedent.
//!
//! The explicit `organization_id = $N` SQL filters are retained as
//! defense-in-depth; handlers pass `rls.tenant_id()` (the org the caller was
//! validated against), so the SQL filter and the RLS context can never disagree.

use crate::models::{
    CreateEquipment, CreateMaintenance, Equipment, EquipmentMaintenance, EquipmentQuery,
    MaintenancePrediction, UpdateEquipment, UpdateMaintenance,
};
use chrono::NaiveDate;
use sqlx::{Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

/// Repository for equipment and maintenance operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct EquipmentRepository;

impl EquipmentRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    /// Create equipment.
    ///
    /// `organization_id` is passed explicitly by the caller and must originate
    /// from the verified request principal, never from client input in `data`.
    pub async fn create<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        data: CreateEquipment,
    ) -> Result<Equipment, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO equipment
                (organization_id, building_id, facility_id, name, category, manufacturer, model,
                 serial_number, installation_date, warranty_expires, expected_lifespan_years,
                 maintenance_interval_days, notes, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(data.facility_id)
        .bind(data.name)
        .bind(data.category)
        .bind(data.manufacturer)
        .bind(data.model)
        .bind(data.serial_number)
        .bind(data.installation_date)
        .bind(data.warranty_expires)
        .bind(data.expected_lifespan_years)
        .bind(data.maintenance_interval_days)
        .bind(data.notes)
        .bind(sqlx::types::Json(data.metadata.unwrap_or_default()))
        .fetch_one(executor)
        .await
    }

    /// Fetch a single equipment row scoped to `org_id`.
    ///
    /// Returns `None` for both "not found" and "belongs to another tenant" so
    /// callers cannot enumerate foreign-tenant IDs by status code. RLS scopes
    /// the result to the connection's org context as well.
    pub async fn find_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Equipment>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM equipment WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .fetch_optional(executor)
            .await
    }

    /// List equipment with query filters.
    pub async fn list<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: EquipmentQuery,
    ) -> Result<Vec<Equipment>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT * FROM equipment
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::uuid IS NULL OR facility_id = $3)
                AND ($4::text IS NULL OR category = $4)
                AND ($5::text IS NULL OR status = $5)
                AND ($6::boolean IS NULL OR ($6 = TRUE AND next_maintenance_due <= CURRENT_DATE))
            ORDER BY name
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.facility_id)
        .bind(query.category)
        .bind(query.status)
        .bind(query.needs_maintenance)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update equipment — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal so that a
    /// caller in org B cannot modify equipment belonging to org A.  The WHERE
    /// clause `AND organization_id = $14` makes the update a no-op (returns
    /// `RowNotFound`) when the row exists but belongs to a different tenant.
    pub async fn update<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateEquipment,
    ) -> Result<Equipment, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE equipment SET
                name = COALESCE($2, name),
                category = COALESCE($3, category),
                manufacturer = COALESCE($4, manufacturer),
                model = COALESCE($5, model),
                serial_number = COALESCE($6, serial_number),
                installation_date = COALESCE($7, installation_date),
                warranty_expires = COALESCE($8, warranty_expires),
                expected_lifespan_years = COALESCE($9, expected_lifespan_years),
                maintenance_interval_days = COALESCE($10, maintenance_interval_days),
                status = COALESCE($11, status),
                notes = COALESCE($12, notes),
                metadata = COALESCE($13, metadata),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $14
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.name)
        .bind(data.category)
        .bind(data.manufacturer)
        .bind(data.model)
        .bind(data.serial_number)
        .bind(data.installation_date)
        .bind(data.warranty_expires)
        .bind(data.expected_lifespan_years)
        .bind(data.maintenance_interval_days)
        .bind(data.status)
        .bind(data.notes)
        .bind(data.metadata.map(sqlx::types::Json))
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// Delete equipment — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.  The WHERE
    /// clause `AND organization_id = $2` ensures a caller in org B cannot delete
    /// equipment belonging to org A (returns `false` / not-found rather than
    /// deleting the row).
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM equipment WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Create a maintenance record — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// The INSERT is guarded by a sub-select that ensures `data.equipment_id`
    /// belongs to `org_id`; if the equipment does not exist in that org the
    /// INSERT produces no row and `sqlx` returns `RowNotFound`.
    pub async fn create_maintenance<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateMaintenance,
    ) -> Result<EquipmentMaintenance, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO equipment_maintenance
                (equipment_id, maintenance_type, description, performed_by, external_vendor,
                 cost, parts_replaced, fault_id, scheduled_date, notes)
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            FROM equipment
            WHERE id = $1 AND organization_id = $11
            RETURNING *
            "#,
        )
        .bind(data.equipment_id)
        .bind(data.maintenance_type)
        .bind(data.description)
        .bind(data.performed_by)
        .bind(data.external_vendor)
        .bind(data.cost)
        .bind(data.parts_replaced.unwrap_or_default())
        .bind(data.fault_id)
        .bind(data.scheduled_date)
        .bind(data.notes)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// Get maintenance record by ID — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// The JOIN ensures a caller in org B cannot read a maintenance record
    /// whose parent equipment belongs to org A; returns `None` for both
    /// "not found" and "belongs to another tenant" to prevent ID enumeration.
    pub async fn find_maintenance_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<EquipmentMaintenance>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT em.id, em.equipment_id, em.maintenance_type, em.description,
                em.performed_by, em.external_vendor, em.cost,
                COALESCE(em.parts_replaced, '{}') AS parts_replaced,
                em.fault_id, em.scheduled_date, em.completed_date,
                em.status, em.notes, em.created_at, em.updated_at
            FROM equipment_maintenance em
            JOIN equipment e ON e.id = em.equipment_id
            WHERE em.id = $1 AND e.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List maintenance records for equipment — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// The JOIN ensures only maintenance records whose parent equipment
    /// belongs to `org_id` are returned.
    pub async fn list_maintenance<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        org_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EquipmentMaintenance>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT em.id, em.equipment_id, em.maintenance_type, em.description,
                em.performed_by, em.external_vendor, em.cost,
                COALESCE(em.parts_replaced, '{}') AS parts_replaced,
                em.fault_id, em.scheduled_date, em.completed_date,
                em.status, em.notes, em.created_at, em.updated_at
            FROM equipment_maintenance em
            JOIN equipment e ON e.id = em.equipment_id
            WHERE em.equipment_id = $1 AND e.organization_id = $2
            ORDER BY COALESCE(em.completed_date, em.scheduled_date) DESC NULLS LAST
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update maintenance record — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// `equipment_maintenance` has no direct `organization_id` column; the
    /// tenant boundary is enforced via the parent `equipment` row using an
    /// `EXISTS` sub-select so a caller in org B cannot mutate a maintenance
    /// record whose parent equipment belongs to org A.
    ///
    /// Multi-statement (update + optional last-maintenance refresh), so it takes
    /// a connection and reborrows it for each query.
    pub async fn update_maintenance(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        org_id: Uuid,
        data: UpdateMaintenance,
    ) -> Result<EquipmentMaintenance, sqlx::Error> {
        let result: EquipmentMaintenance = sqlx::query_as(
            r#"
            UPDATE equipment_maintenance SET
                maintenance_type = COALESCE($2, maintenance_type),
                description = COALESCE($3, description),
                performed_by = COALESCE($4, performed_by),
                external_vendor = COALESCE($5, external_vendor),
                cost = COALESCE($6, cost),
                parts_replaced = COALESCE($7, parts_replaced),
                completed_date = COALESCE($8, completed_date),
                status = COALESCE($9, status),
                notes = COALESCE($10, notes),
                updated_at = NOW()
            WHERE id = $1
              AND EXISTS (
                  SELECT 1 FROM equipment e
                  WHERE e.id = equipment_maintenance.equipment_id
                    AND e.organization_id = $11
              )
            RETURNING
                id, equipment_id, maintenance_type, description, performed_by, external_vendor,
                cost, COALESCE(parts_replaced, '{}') AS parts_replaced, fault_id,
                scheduled_date, completed_date, status, notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(data.maintenance_type)
        .bind(data.description)
        .bind(data.performed_by)
        .bind(data.external_vendor)
        .bind(data.cost)
        .bind(data.parts_replaced)
        .bind(data.completed_date)
        .bind(data.status)
        .bind(data.notes)
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        // Update equipment last maintenance date if completed
        if result.status == "completed" {
            if let Some(completed) = result.completed_date {
                self.update_last_maintenance(&mut *conn, result.equipment_id, completed)
                    .await?;
            }
        }

        Ok(result)
    }

    /// Update equipment's last maintenance date.
    async fn update_last_maintenance<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        date: NaiveDate,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE equipment
            SET last_maintenance_date = $2,
                next_maintenance_due = CASE
                    WHEN maintenance_interval_days IS NOT NULL
                    THEN $2 + (maintenance_interval_days || ' days')::interval
                    ELSE next_maintenance_due
                END,
                status = 'operational',
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(equipment_id)
        .bind(date)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Create or update a maintenance prediction.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_prediction<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        risk_score: f64,
        predicted_failure_date: Option<NaiveDate>,
        confidence: f64,
        recommendation: &str,
        factors: serde_json::Value,
    ) -> Result<MaintenancePrediction, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO maintenance_predictions
                (equipment_id, risk_score, predicted_failure_date, confidence, recommendation, factors)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (equipment_id) DO UPDATE SET
                risk_score = EXCLUDED.risk_score,
                predicted_failure_date = EXCLUDED.predicted_failure_date,
                confidence = EXCLUDED.confidence,
                recommendation = EXCLUDED.recommendation,
                factors = EXCLUDED.factors,
                acknowledged = FALSE,
                acknowledged_by = NULL,
                action_taken = NULL,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(equipment_id)
        .bind(risk_score)
        .bind(predicted_failure_date)
        .bind(confidence)
        .bind(recommendation)
        .bind(sqlx::types::Json(factors))
        .fetch_one(executor)
        .await
    }

    /// Get high-risk predictions.
    pub async fn list_high_risk_predictions<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        min_risk_score: f64,
        limit: i64,
    ) -> Result<Vec<MaintenancePrediction>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT p.* FROM maintenance_predictions p
            JOIN equipment e ON e.id = p.equipment_id
            WHERE e.organization_id = $1 AND p.risk_score >= $2
            ORDER BY p.risk_score DESC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(min_risk_score)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Acknowledge a prediction — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// The EXISTS sub-select ensures a caller in org B cannot acknowledge a
    /// prediction whose parent equipment belongs to org A; `fetch_one` returns
    /// `RowNotFound` when either the prediction does not exist or the equipment
    /// belongs to a different tenant (callers should surface both as 404 to
    /// prevent ID enumeration).
    pub async fn acknowledge_prediction<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        action_taken: Option<&str>,
    ) -> Result<MaintenancePrediction, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE maintenance_predictions
            SET acknowledged = TRUE, acknowledged_by = $3, action_taken = $4, updated_at = NOW()
            WHERE id = $1
              AND EXISTS (
                SELECT 1 FROM equipment e
                WHERE e.id = maintenance_predictions.equipment_id
                  AND e.organization_id = $2
              )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .bind(action_taken)
        .fetch_one(executor)
        .await
    }

    /// Get equipment needing maintenance soon.
    pub async fn list_needing_maintenance<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        days_ahead: i32,
        limit: i64,
    ) -> Result<Vec<Equipment>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM equipment
            WHERE organization_id = $1
                AND next_maintenance_due IS NOT NULL
                AND next_maintenance_due <= CURRENT_DATE + ($2 || ' days')::interval
                AND status != 'decommissioned'
            ORDER BY next_maintenance_due
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(days_ahead)
        .bind(limit)
        .fetch_all(executor)
        .await
    }
}

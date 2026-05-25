//! Equipment and predictive maintenance repository (Epic 13, Story 13.3).

use crate::models::{
    CreateEquipment, CreateMaintenance, Equipment, EquipmentMaintenance, EquipmentQuery,
    MaintenancePrediction, UpdateEquipment, UpdateMaintenance,
};
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

/// Repository for equipment and maintenance operations.
#[derive(Clone)]
pub struct EquipmentRepository {
    pool: PgPool,
}

impl EquipmentRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create new equipment.
    /// Create equipment.
    ///
    /// `organization_id` is passed explicitly by the caller and must originate
    /// from the verified request principal, never from client input in `data`.
    pub async fn create(
        &self,
        organization_id: Uuid,
        data: CreateEquipment,
    ) -> Result<Equipment, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await
    }

    /// Get equipment by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Equipment>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM equipment WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List equipment with query filters.
    pub async fn list(
        &self,
        org_id: Uuid,
        query: EquipmentQuery,
    ) -> Result<Vec<Equipment>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await
    }

    /// Update equipment — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal so that a
    /// caller in org B cannot modify equipment belonging to org A.  The WHERE
    /// clause `AND organization_id = $14` makes the update a no-op (returns
    /// `RowNotFound`) when the row exists but belongs to a different tenant.
    pub async fn update(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: UpdateEquipment,
    ) -> Result<Equipment, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await
    }

    /// Delete equipment — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.  The WHERE
    /// clause `AND organization_id = $2` ensures a caller in org B cannot delete
    /// equipment belonging to org A (returns `false` / not-found rather than
    /// deleting the row).
    pub async fn delete(&self, id: Uuid, org_id: Uuid) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM equipment WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Create maintenance record.
    pub async fn create_maintenance(
        &self,
        data: CreateMaintenance,
    ) -> Result<EquipmentMaintenance, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO equipment_maintenance
                (equipment_id, maintenance_type, description, performed_by, external_vendor,
                 cost, parts_replaced, fault_id, scheduled_date, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        .fetch_one(&self.pool)
        .await
    }

    /// Get maintenance record by ID.
    pub async fn find_maintenance_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<EquipmentMaintenance>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM equipment_maintenance WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List maintenance records for equipment.
    pub async fn list_maintenance(
        &self,
        equipment_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EquipmentMaintenance>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT * FROM equipment_maintenance
            WHERE equipment_id = $1
            ORDER BY COALESCE(completed_date, scheduled_date) DESC NULLS LAST
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(equipment_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update maintenance record — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// `equipment_maintenance` has no direct `organization_id` column; the
    /// tenant boundary is enforced via the parent `equipment` row using an
    /// `EXISTS` sub-select so a caller in org B cannot mutate a maintenance
    /// record whose parent equipment belongs to org A.
    pub async fn update_maintenance(
        &self,
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
            RETURNING *
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
        .fetch_one(&self.pool)
        .await?;

        // Update equipment last maintenance date if completed
        if result.status == "completed" {
            if let Some(completed) = result.completed_date {
                self.update_last_maintenance(result.equipment_id, completed)
                    .await?;
            }
        }

        Ok(result)
    }

    /// Update equipment's last maintenance date.
    async fn update_last_maintenance(
        &self,
        equipment_id: Uuid,
        date: NaiveDate,
    ) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create or update a maintenance prediction.
    pub async fn upsert_prediction(
        &self,
        equipment_id: Uuid,
        risk_score: f64,
        predicted_failure_date: Option<NaiveDate>,
        confidence: f64,
        recommendation: &str,
        factors: serde_json::Value,
    ) -> Result<MaintenancePrediction, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await
    }

    /// Get high-risk predictions.
    pub async fn list_high_risk_predictions(
        &self,
        org_id: Uuid,
        min_risk_score: f64,
        limit: i64,
    ) -> Result<Vec<MaintenancePrediction>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await
    }

    /// Acknowledge a prediction.
    pub async fn acknowledge_prediction(
        &self,
        id: Uuid,
        user_id: Uuid,
        action_taken: Option<&str>,
    ) -> Result<MaintenancePrediction, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE maintenance_predictions
            SET acknowledged = TRUE, acknowledged_by = $2, action_taken = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(action_taken)
        .fetch_one(&self.pool)
        .await
    }

    /// Get equipment needing maintenance soon.
    pub async fn list_needing_maintenance(
        &self,
        org_id: Uuid,
        days_ahead: i32,
        limit: i64,
    ) -> Result<Vec<Equipment>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await
    }
}

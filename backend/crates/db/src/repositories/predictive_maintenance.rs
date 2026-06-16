//! Repository for Epic 134: Predictive Maintenance & Equipment Intelligence.
//!
//! # RLS Integration (PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the predictive-maintenance tables
//! (`equipment_registry`, `equipment_documents`, `maintenance_logs`,
//! `maintenance_log_photos`, `equipment_predictions`, `maintenance_alerts`,
//! `equipment_health_thresholds`). Under `FORCE` the api-server's owner
//! connection is no longer exempt, so a query issued on a connection without
//! `app.current_org_id` set collapses to deny-all (own-org reads return empty,
//! writes fail the policy's `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` / `document.rs` precedent.
//!
//! Single-statement methods take a generic `Executor`; the multi-statement
//! methods (`run_prediction`, `get_dashboard`) take `&mut PgConnection` and
//! reborrow it for each query.

use chrono::Utc;
use common::AppError;
use rust_decimal::Decimal;
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

use crate::models::predictive_maintenance::{
    AlertWithEquipment, AlertsBySeverity, CreateEquipment, CreateEquipmentDocument,
    CreateMaintenanceLog, Equipment, EquipmentByStatus, EquipmentDocument, EquipmentPrediction,
    EquipmentQuery, EquipmentSummary, HealthDistribution, HealthThreshold, MaintenanceAlert,
    MaintenanceDashboard, MaintenanceLog, MaintenanceLogPhoto, PredictionFactor, PredictionResult,
    SetHealthThreshold, UpdateEquipment, UpdateMaintenanceLog,
};

/// Repository for predictive maintenance operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Debug, Clone)]
pub struct PredictiveMaintenanceRepository;

impl PredictiveMaintenanceRepository {
    /// Create a new repository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: crate::DbPool) -> Self {
        Self
    }

    // ========================================================================
    // EQUIPMENT REGISTRY (Story 134.1)
    // ========================================================================

    /// Create new equipment.
    pub async fn create_equipment<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateEquipment,
    ) -> Result<Equipment, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let equipment = sqlx::query_as::<_, Equipment>(
            r#"
            INSERT INTO equipment_registry (
                organization_id, building_id, name, equipment_type, category,
                manufacturer, model, serial_number, location_description,
                floor_number, unit_id, installation_date, warranty_expiry_date,
                expected_lifespan_years, replacement_cost, notes, specifications,
                status, created_by, updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 'operational', $18, $18)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(&req.name)
        .bind(&req.equipment_type)
        .bind(&req.category)
        .bind(&req.manufacturer)
        .bind(&req.model)
        .bind(&req.serial_number)
        .bind(&req.location_description)
        .bind(req.floor_number)
        .bind(req.unit_id)
        .bind(req.installation_date)
        .bind(req.warranty_expiry_date)
        .bind(req.expected_lifespan_years)
        .bind(req.replacement_cost)
        .bind(&req.notes)
        .bind(&req.specifications)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(equipment)
    }

    /// Get equipment by ID.
    ///
    /// RLS scopes the result to the connection's org context, so equipment
    /// owned by another organization is invisible (returns `None`).
    pub async fn get_equipment<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Equipment>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let equipment = sqlx::query_as::<_, Equipment>(
            r#"
            SELECT * FROM equipment_registry
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(equipment)
    }

    /// List equipment with filters.
    pub async fn list_equipment<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: EquipmentQuery,
    ) -> Result<Vec<Equipment>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        let sort_by = query.sort_by.unwrap_or_else(|| "name".to_string());
        let sort_order = query.sort_order.unwrap_or_else(|| "asc".to_string());

        // Build dynamic ORDER BY from an ALLOWLIST of column + direction.
        // Both the sort column and the sort direction are mapped through a
        // `match` to fixed string literals — never interpolated from the raw
        // request value — so a malicious `sort_by`/`sort_order` (e.g.
        // `id; DROP TABLE ...` or `(SELECT ...)`) can never reach the SQL text.
        // Anything outside the allowlist falls back to the default.
        let sort_column = match sort_by.as_str() {
            "health_score" => "health_score",
            "next_predicted_failure" => "next_predicted_failure",
            _ => "name",
        };
        let sort_direction = match sort_order.to_ascii_lowercase().as_str() {
            "desc" => "DESC",
            _ => "ASC",
        };
        let order_clause = format!("{sort_column} {sort_direction}");

        let sql = format!(
            r#"
            SELECT * FROM equipment_registry
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
              AND ($3::text IS NULL OR equipment_type = $3)
              AND ($4::text IS NULL OR status = $4)
              AND ($5::int IS NULL OR health_score >= $5)
              AND ($6::int IS NULL OR health_score <= $6)
            ORDER BY {}
            LIMIT $7 OFFSET $8
            "#,
            order_clause
        );
        let equipment = sqlx::query_as::<_, Equipment>(sqlx::AssertSqlSafe(sql))
            .bind(org_id)
            .bind(query.building_id)
            .bind(&query.equipment_type)
            .bind(&query.status)
            .bind(query.min_health_score)
            .bind(query.max_health_score)
            .bind(limit)
            .bind(offset)
            .fetch_all(executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(equipment)
    }

    /// Update equipment.
    pub async fn update_equipment<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        req: UpdateEquipment,
    ) -> Result<Option<Equipment>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let equipment = sqlx::query_as::<_, Equipment>(
            r#"
            UPDATE equipment_registry SET
                name = COALESCE($3, name),
                equipment_type = COALESCE($4, equipment_type),
                category = COALESCE($5, category),
                manufacturer = COALESCE($6, manufacturer),
                model = COALESCE($7, model),
                serial_number = COALESCE($8, serial_number),
                location_description = COALESCE($9, location_description),
                floor_number = COALESCE($10, floor_number),
                unit_id = COALESCE($11, unit_id),
                installation_date = COALESCE($12, installation_date),
                warranty_expiry_date = COALESCE($13, warranty_expiry_date),
                expected_lifespan_years = COALESCE($14, expected_lifespan_years),
                replacement_cost = COALESCE($15, replacement_cost),
                status = COALESCE($16, status),
                notes = COALESCE($17, notes),
                specifications = COALESCE($18, specifications),
                updated_at = NOW(),
                updated_by = $19
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.equipment_type)
        .bind(&req.category)
        .bind(&req.manufacturer)
        .bind(&req.model)
        .bind(&req.serial_number)
        .bind(&req.location_description)
        .bind(req.floor_number)
        .bind(req.unit_id)
        .bind(req.installation_date)
        .bind(req.warranty_expiry_date)
        .bind(req.expected_lifespan_years)
        .bind(req.replacement_cost)
        .bind(&req.status)
        .bind(&req.notes)
        .bind(&req.specifications)
        .bind(user_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(equipment)
    }

    /// Delete equipment.
    pub async fn delete_equipment<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM equipment_registry
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Add document to equipment.
    pub async fn add_equipment_document<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateEquipmentDocument,
    ) -> Result<EquipmentDocument, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let doc = sqlx::query_as::<_, EquipmentDocument>(
            r#"
            INSERT INTO equipment_documents (
                equipment_id, organization_id, document_type, title,
                file_path, file_size, mime_type, uploaded_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(&req.document_type)
        .bind(&req.title)
        .bind(&req.file_path)
        .bind(req.file_size)
        .bind(&req.mime_type)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(doc)
    }

    /// List equipment documents.
    pub async fn list_equipment_documents<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<EquipmentDocument>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let docs = sqlx::query_as::<_, EquipmentDocument>(
            r#"
            SELECT * FROM equipment_documents
            WHERE equipment_id = $1 AND organization_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(docs)
    }

    // ========================================================================
    // MAINTENANCE LOGS (Story 134.2)
    // ========================================================================

    /// Create maintenance log.
    pub async fn create_maintenance_log<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateMaintenanceLog,
    ) -> Result<MaintenanceLog, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let log = sqlx::query_as::<_, MaintenanceLog>(
            r#"
            INSERT INTO maintenance_logs (
                equipment_id, organization_id, maintenance_type, description,
                scheduled_date, started_at, completed_at, duration_minutes,
                cost, currency, vendor_id, vendor_name, technician_name,
                parts_replaced, work_performed, fault_id, work_order_id,
                outcome, notes, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            RETURNING *
            "#,
        )
        .bind(req.equipment_id)
        .bind(org_id)
        .bind(&req.maintenance_type)
        .bind(&req.description)
        .bind(req.scheduled_date)
        .bind(req.started_at)
        .bind(req.completed_at)
        .bind(req.duration_minutes)
        .bind(req.cost)
        .bind(&req.currency)
        .bind(req.vendor_id)
        .bind(&req.vendor_name)
        .bind(&req.technician_name)
        .bind(&req.parts_replaced)
        .bind(&req.work_performed)
        .bind(req.fault_id)
        .bind(req.work_order_id)
        .bind(&req.outcome)
        .bind(&req.notes)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(log)
    }

    /// Get maintenance log by ID.
    ///
    /// RLS scopes the result to the connection's org context, so a log owned by
    /// another organization is invisible (returns `None`).
    pub async fn get_maintenance_log<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MaintenanceLog>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let log = sqlx::query_as::<_, MaintenanceLog>(
            r#"
            SELECT * FROM maintenance_logs
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(log)
    }

    /// List maintenance logs for equipment.
    pub async fn list_maintenance_logs<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        org_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MaintenanceLog>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let logs = sqlx::query_as::<_, MaintenanceLog>(
            r#"
            SELECT * FROM maintenance_logs
            WHERE equipment_id = $1 AND organization_id = $2
            ORDER BY COALESCE(completed_at, scheduled_date, created_at) DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(logs)
    }

    /// Update maintenance log.
    pub async fn update_maintenance_log<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        req: UpdateMaintenanceLog,
    ) -> Result<Option<MaintenanceLog>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let log = sqlx::query_as::<_, MaintenanceLog>(
            r#"
            UPDATE maintenance_logs SET
                maintenance_type = COALESCE($3, maintenance_type),
                description = COALESCE($4, description),
                scheduled_date = COALESCE($5, scheduled_date),
                started_at = COALESCE($6, started_at),
                completed_at = COALESCE($7, completed_at),
                duration_minutes = COALESCE($8, duration_minutes),
                cost = COALESCE($9, cost),
                currency = COALESCE($10, currency),
                vendor_name = COALESCE($11, vendor_name),
                technician_name = COALESCE($12, technician_name),
                parts_replaced = COALESCE($13, parts_replaced),
                work_performed = COALESCE($14, work_performed),
                outcome = COALESCE($15, outcome),
                notes = COALESCE($16, notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&req.maintenance_type)
        .bind(&req.description)
        .bind(req.scheduled_date)
        .bind(req.started_at)
        .bind(req.completed_at)
        .bind(req.duration_minutes)
        .bind(req.cost)
        .bind(&req.currency)
        .bind(&req.vendor_name)
        .bind(&req.technician_name)
        .bind(&req.parts_replaced)
        .bind(&req.work_performed)
        .bind(&req.outcome)
        .bind(&req.notes)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(log)
    }

    /// Add photo to maintenance log.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_maintenance_photo<'e, E>(
        &self,
        executor: E,
        log_id: Uuid,
        user_id: Uuid,
        file_path: &str,
        file_size: Option<i32>,
        mime_type: Option<&str>,
        caption: Option<&str>,
        photo_type: Option<&str>,
    ) -> Result<MaintenanceLogPhoto, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let photo = sqlx::query_as::<_, MaintenanceLogPhoto>(
            r#"
            INSERT INTO maintenance_log_photos (
                maintenance_log_id, file_path, file_size, mime_type,
                caption, photo_type, uploaded_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(log_id)
        .bind(file_path)
        .bind(file_size)
        .bind(mime_type)
        .bind(caption)
        .bind(photo_type)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(photo)
    }

    /// List photos for a maintenance log, scoped to the caller's organization.
    ///
    /// `maintenance_log_photos` has no `organization_id` column, so org
    /// ownership is enforced by joining through the parent `maintenance_logs`
    /// row (which is org-scoped, and under RLS only the caller's org rows are
    /// visible). A `log_id` belonging to another org yields zero rows — the
    /// handler must surface that as a 404, never a cross-tenant read (IDOR #848).
    pub async fn list_maintenance_photos<'e, E>(
        &self,
        executor: E,
        log_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MaintenanceLogPhoto>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let photos = sqlx::query_as::<_, MaintenanceLogPhoto>(
            r#"
            SELECT p.*
            FROM maintenance_log_photos p
            JOIN maintenance_logs ml ON ml.id = p.maintenance_log_id
            WHERE p.maintenance_log_id = $1
              AND ml.organization_id = $2
            ORDER BY p.created_at
            "#,
        )
        .bind(log_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(photos)
    }

    // ========================================================================
    // PREDICTIONS (Story 134.3)
    // ========================================================================

    /// Run prediction for equipment (stub implementation).
    ///
    /// Multi-statement: reads equipment + maintenance history, then writes a
    /// prediction row and updates the equipment health score. Takes a
    /// `&mut PgConnection` and reborrows it for each query so every statement
    /// runs under the same RLS context.
    pub async fn run_prediction(
        &self,
        conn: &mut PgConnection,
        equipment_id: Uuid,
        org_id: Uuid,
    ) -> Result<PredictionResult, AppError> {
        // Get equipment info
        let equipment = self
            .get_equipment(&mut *conn, equipment_id, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Equipment not found".to_string()))?;

        // Calculate age-based factors
        let age_years = equipment
            .installation_date
            .map(|d| {
                let now = Utc::now().date_naive();
                (now - d).num_days() as f64 / 365.0
            })
            .unwrap_or(0.0);

        let expected_lifespan = equipment.expected_lifespan_years.unwrap_or(15) as f64;
        let age_factor = (age_years / expected_lifespan).min(1.0);

        // Get maintenance history count
        let maintenance_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM maintenance_logs
            WHERE equipment_id = $1 AND organization_id = $2
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Calculate health score (simplified model)
        let base_health = 100.0 - (age_factor * 60.0);
        let maintenance_bonus = (maintenance_count as f64 * 2.0).min(20.0);
        let health_score = ((base_health + maintenance_bonus) as i32).clamp(0, 100);

        // Calculate failure probability
        let failure_probability = (age_factor * 0.8).clamp(0.0, 0.99);

        // Determine recommended action
        let (recommended_action, urgency) = match health_score {
            0..=20 => ("replace", "critical"),
            21..=40 => ("schedule_maintenance", "high"),
            41..=60 => ("schedule_maintenance", "medium"),
            61..=80 => ("monitor", "low"),
            _ => ("none", "low"),
        };

        // Calculate predicted failure date
        let predicted_failure_date = if failure_probability > 0.3 {
            let days_to_failure = ((1.0 - failure_probability) * 365.0) as i64;
            Some(Utc::now().date_naive() + chrono::Duration::days(days_to_failure))
        } else {
            None
        };

        // Build factors
        let factors = vec![
            PredictionFactor {
                factor: "age".to_string(),
                weight: 0.4,
                value: format!("{:.1} years", age_years),
                description: Some("Equipment age relative to expected lifespan".to_string()),
            },
            PredictionFactor {
                factor: "maintenance_history".to_string(),
                weight: 0.3,
                value: format!("{} records", maintenance_count),
                description: Some("Number of maintenance activities logged".to_string()),
            },
            PredictionFactor {
                factor: "equipment_type".to_string(),
                weight: 0.2,
                value: equipment.equipment_type.clone(),
                description: Some("Type-specific failure patterns".to_string()),
            },
            PredictionFactor {
                factor: "warranty_status".to_string(),
                weight: 0.1,
                value: equipment
                    .warranty_expiry_date
                    .map(|d| {
                        if d > Utc::now().date_naive() {
                            "active".to_string()
                        } else {
                            "expired".to_string()
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string()),
                description: Some("Warranty coverage status".to_string()),
            },
        ];

        // Save prediction to history
        let factors_json = serde_json::to_value(&factors).unwrap_or_default();
        sqlx::query(
            r#"
            INSERT INTO equipment_predictions (
                equipment_id, organization_id, health_score, failure_probability,
                predicted_failure_date, confidence_level, factors, model_version,
                model_type, recommended_action, recommended_date, urgency
            )
            VALUES ($1, $2, $3, $4, $5, 0.75, $6, '1.0.0', 'time_based', $7, $8, $9)
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(health_score)
        .bind(Decimal::from_f64_retain(failure_probability).unwrap_or_default())
        .bind(predicted_failure_date)
        .bind(&factors_json)
        .bind(recommended_action)
        .bind(predicted_failure_date)
        .bind(urgency)
        .execute(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Update equipment health score
        sqlx::query(
            r#"
            UPDATE equipment_registry SET
                health_score = $3,
                failure_probability = $4,
                next_predicted_failure = $5,
                last_prediction_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(health_score)
        .bind(Decimal::from_f64_retain(failure_probability).unwrap_or_default())
        .bind(predicted_failure_date)
        .execute(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(PredictionResult {
            equipment_id,
            equipment_name: equipment.name,
            health_score,
            failure_probability,
            predicted_failure_date,
            recommended_action: recommended_action.to_string(),
            urgency: urgency.to_string(),
            factors,
        })
    }

    /// Get prediction history for equipment.
    pub async fn get_prediction_history<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        org_id: Uuid,
        limit: i64,
    ) -> Result<Vec<EquipmentPrediction>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let predictions = sqlx::query_as::<_, EquipmentPrediction>(
            r#"
            SELECT * FROM equipment_predictions
            WHERE equipment_id = $1 AND organization_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(limit)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(predictions)
    }

    // ========================================================================
    // ALERTS
    // ========================================================================

    /// Create maintenance alert.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_alert<'e, E>(
        &self,
        executor: E,
        equipment_id: Uuid,
        org_id: Uuid,
        prediction_id: Option<Uuid>,
        alert_type: &str,
        severity: &str,
        title: &str,
        message: &str,
    ) -> Result<MaintenanceAlert, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let alert = sqlx::query_as::<_, MaintenanceAlert>(
            r#"
            INSERT INTO maintenance_alerts (
                equipment_id, organization_id, prediction_id,
                alert_type, severity, title, message, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active')
            RETURNING *
            "#,
        )
        .bind(equipment_id)
        .bind(org_id)
        .bind(prediction_id)
        .bind(alert_type)
        .bind(severity)
        .bind(title)
        .bind(message)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alert)
    }

    /// List active alerts.
    pub async fn list_alerts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        status: Option<&str>,
        severity: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AlertWithEquipment>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let alerts = sqlx::query_as::<_, AlertWithEquipment>(
            r#"
            SELECT
                a.id, a.equipment_id,
                e.name as equipment_name,
                b.name as building_name,
                a.alert_type, a.severity, a.title, a.message,
                a.status, a.created_at
            FROM maintenance_alerts a
            JOIN equipment_registry e ON a.equipment_id = e.id
            JOIN buildings b ON e.building_id = b.id
            WHERE a.organization_id = $1
              AND ($2::text IS NULL OR a.status = $2)
              AND ($3::text IS NULL OR a.severity = $3)
            ORDER BY
                CASE a.severity
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    ELSE 4
                END,
                a.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(status)
        .bind(severity)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alerts)
    }

    /// Acknowledge alert.
    pub async fn acknowledge_alert<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MaintenanceAlert>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let alert = sqlx::query_as::<_, MaintenanceAlert>(
            r#"
            UPDATE maintenance_alerts SET
                status = 'acknowledged',
                acknowledged_at = NOW(),
                acknowledged_by = $3
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alert)
    }

    /// Resolve alert.
    pub async fn resolve_alert<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        maintenance_log_id: Option<Uuid>,
    ) -> Result<Option<MaintenanceAlert>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let alert = sqlx::query_as::<_, MaintenanceAlert>(
            r#"
            UPDATE maintenance_alerts SET
                status = 'resolved',
                resolved_at = NOW(),
                resolved_by = $3,
                maintenance_log_id = $4
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .bind(maintenance_log_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alert)
    }

    /// Dismiss alert.
    pub async fn dismiss_alert<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MaintenanceAlert>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let alert = sqlx::query_as::<_, MaintenanceAlert>(
            r#"
            UPDATE maintenance_alerts SET
                status = 'dismissed'
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alert)
    }

    // ========================================================================
    // HEALTH THRESHOLDS
    // ========================================================================

    /// Set health threshold.
    pub async fn set_health_threshold<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        req: SetHealthThreshold,
    ) -> Result<HealthThreshold, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let threshold = sqlx::query_as::<_, HealthThreshold>(
            r#"
            INSERT INTO equipment_health_thresholds (
                organization_id, equipment_type, critical_threshold,
                warning_threshold, alert_on_critical, alert_on_warning
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (organization_id, equipment_type) DO UPDATE SET
                critical_threshold = EXCLUDED.critical_threshold,
                warning_threshold = EXCLUDED.warning_threshold,
                alert_on_critical = EXCLUDED.alert_on_critical,
                alert_on_warning = EXCLUDED.alert_on_warning,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&req.equipment_type)
        .bind(req.critical_threshold)
        .bind(req.warning_threshold)
        .bind(req.alert_on_critical.unwrap_or(true))
        .bind(req.alert_on_warning.unwrap_or(true))
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(threshold)
    }

    /// List health thresholds.
    pub async fn list_health_thresholds<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<HealthThreshold>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let thresholds = sqlx::query_as::<_, HealthThreshold>(
            r#"
            SELECT * FROM equipment_health_thresholds
            WHERE organization_id = $1
            ORDER BY equipment_type
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(thresholds)
    }

    // ========================================================================
    // DASHBOARD (Story 134.4)
    // ========================================================================

    /// Get maintenance dashboard data.
    ///
    /// Multi-statement read: issues several aggregate queries on one connection,
    /// reborrowing `&mut *conn` for each so they all run under the same RLS
    /// context.
    pub async fn get_dashboard(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<MaintenanceDashboard, AppError> {
        // Total equipment count
        let total_equipment: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM equipment_registry
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Equipment by status
        let status_counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*) as count
            FROM equipment_registry
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
            GROUP BY status
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut equipment_by_status = EquipmentByStatus {
            operational: 0,
            needs_maintenance: 0,
            under_repair: 0,
            decommissioned: 0,
        };
        for (status, count) in status_counts {
            match status.as_str() {
                "operational" => equipment_by_status.operational = count as i32,
                "needs_maintenance" => equipment_by_status.needs_maintenance = count as i32,
                "under_repair" => equipment_by_status.under_repair = count as i32,
                "decommissioned" => equipment_by_status.decommissioned = count as i32,
                _ => {}
            }
        }

        // Health distribution
        let health_counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT
                CASE
                    WHEN health_score >= 80 THEN 'excellent'
                    WHEN health_score >= 60 THEN 'good'
                    WHEN health_score >= 40 THEN 'fair'
                    WHEN health_score >= 20 THEN 'poor'
                    ELSE 'critical'
                END as health_category,
                COUNT(*) as count
            FROM equipment_registry
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
              AND health_score IS NOT NULL
            GROUP BY health_category
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut health_distribution = HealthDistribution {
            excellent: 0,
            good: 0,
            fair: 0,
            poor: 0,
            critical: 0,
        };
        for (category, count) in health_counts {
            match category.as_str() {
                "excellent" => health_distribution.excellent = count as i32,
                "good" => health_distribution.good = count as i32,
                "fair" => health_distribution.fair = count as i32,
                "poor" => health_distribution.poor = count as i32,
                "critical" => health_distribution.critical = count as i32,
                _ => {}
            }
        }

        // Active alerts count
        let active_alerts: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM maintenance_alerts a
            JOIN equipment_registry e ON a.equipment_id = e.id
            WHERE a.organization_id = $1
              AND a.status = 'active'
              AND ($2::uuid IS NULL OR e.building_id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Alerts by severity
        let severity_counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT a.severity, COUNT(*) as count
            FROM maintenance_alerts a
            JOIN equipment_registry e ON a.equipment_id = e.id
            WHERE a.organization_id = $1
              AND a.status = 'active'
              AND ($2::uuid IS NULL OR e.building_id = $2)
            GROUP BY a.severity
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut alerts_by_severity = AlertsBySeverity {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
        };
        for (severity, count) in severity_counts {
            match severity.as_str() {
                "critical" => alerts_by_severity.critical = count as i32,
                "high" => alerts_by_severity.high = count as i32,
                "medium" => alerts_by_severity.medium = count as i32,
                "low" => alerts_by_severity.low = count as i32,
                _ => {}
            }
        }

        // Upcoming maintenance (scheduled in next 30 days)
        let upcoming_maintenance: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM maintenance_logs ml
            JOIN equipment_registry e ON ml.equipment_id = e.id
            WHERE ml.organization_id = $1
              AND ($2::uuid IS NULL OR e.building_id = $2)
              AND ml.scheduled_date >= CURRENT_DATE
              AND ml.scheduled_date <= CURRENT_DATE + INTERVAL '30 days'
              AND ml.completed_at IS NULL
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Overdue maintenance
        let overdue_maintenance: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM maintenance_logs ml
            JOIN equipment_registry e ON ml.equipment_id = e.id
            WHERE ml.organization_id = $1
              AND ($2::uuid IS NULL OR e.building_id = $2)
              AND ml.scheduled_date < CURRENT_DATE
              AND ml.completed_at IS NULL
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Critical equipment (health_score <= 40)
        let critical_equipment = sqlx::query_as::<_, EquipmentSummary>(
            r#"
            SELECT
                e.id, e.name, e.equipment_type,
                b.name as building_name,
                e.health_score, e.status,
                e.next_predicted_failure, e.failure_probability
            FROM equipment_registry e
            JOIN buildings b ON e.building_id = b.id
            WHERE e.organization_id = $1
              AND ($2::uuid IS NULL OR e.building_id = $2)
              AND e.health_score IS NOT NULL
              AND e.health_score <= 40
            ORDER BY e.health_score ASC
            LIMIT 10
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(MaintenanceDashboard {
            total_equipment: total_equipment as i32,
            equipment_by_status,
            health_distribution,
            active_alerts: active_alerts as i32,
            alerts_by_severity,
            upcoming_maintenance: upcoming_maintenance as i32,
            overdue_maintenance: overdue_maintenance as i32,
            critical_equipment,
        })
    }

    /// Get equipment sorted by health score (for dashboard list).
    pub async fn get_equipment_by_health<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        building_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<EquipmentSummary>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let equipment = sqlx::query_as::<_, EquipmentSummary>(
            r#"
            SELECT
                e.id, e.name, e.equipment_type,
                b.name as building_name,
                e.health_score, e.status,
                e.next_predicted_failure, e.failure_probability
            FROM equipment_registry e
            JOIN buildings b ON e.building_id = b.id
            WHERE e.organization_id = $1
              AND ($2::uuid IS NULL OR e.building_id = $2)
              AND e.health_score IS NOT NULL
            ORDER BY e.health_score ASC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(limit)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(equipment)
    }
}

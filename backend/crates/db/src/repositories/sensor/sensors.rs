//! Sensor CRUD, status, counts & building lookup.

use super::SensorRepository;
use crate::models::{CreateSensor, Sensor, SensorQuery, SensorTypeCount, UpdateSensor};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SensorRepository {
    /// Create a new sensor.
    pub async fn create<'e, E>(
        &self,
        executor: E,
        data: CreateSensor,
    ) -> Result<Sensor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sensors
                (organization_id, building_id, unit_id, name, sensor_type, location,
                 location_description, connection_type, connection_config, unit_of_measurement,
                 data_interval_seconds, manufacturer, model, firmware_version, serial_number,
                 installed_at, metadata, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(data.name)
        .bind(data.sensor_type)
        .bind(data.location)
        .bind(data.location_description)
        .bind(data.connection_type.unwrap_or_else(|| "api".to_string()))
        .bind(sqlx::types::Json(
            data.connection_config.unwrap_or_default(),
        ))
        .bind(data.unit_of_measurement)
        .bind(data.data_interval_seconds)
        .bind(data.manufacturer)
        .bind(data.model)
        .bind(data.firmware_version)
        .bind(data.serial_number)
        .bind(data.installed_at)
        .bind(sqlx::types::Json(data.metadata.unwrap_or_default()))
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Get sensor by ID.
    pub async fn find_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Sensor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM sensors WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List sensors with filters.
    pub async fn list<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: SensorQuery,
    ) -> Result<Vec<Sensor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT * FROM sensors
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::uuid IS NULL OR unit_id = $3)
                AND ($4::text IS NULL OR sensor_type = $4)
                AND ($5::text IS NULL OR status = $5)
                AND ($6::text IS NULL OR name ILIKE '%' || $6 || '%')
            ORDER BY name
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.sensor_type)
        .bind(query.status)
        .bind(query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update a sensor.
    pub async fn update<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSensor,
    ) -> Result<Sensor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensors SET
                name = COALESCE($2, name),
                location = COALESCE($3, location),
                location_description = COALESCE($4, location_description),
                connection_type = COALESCE($5, connection_type),
                connection_config = COALESCE($6, connection_config),
                unit_of_measurement = COALESCE($7, unit_of_measurement),
                data_interval_seconds = COALESCE($8, data_interval_seconds),
                status = COALESCE($9, status),
                manufacturer = COALESCE($10, manufacturer),
                model = COALESCE($11, model),
                firmware_version = COALESCE($12, firmware_version),
                metadata = COALESCE($13, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.name)
        .bind(data.location)
        .bind(data.location_description)
        .bind(data.connection_type)
        .bind(data.connection_config.map(sqlx::types::Json))
        .bind(data.unit_of_measurement)
        .bind(data.data_interval_seconds)
        .bind(data.status)
        .bind(data.manufacturer)
        .bind(data.model)
        .bind(data.firmware_version)
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Delete a sensor.
    pub async fn delete<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM sensors WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update sensor status and timestamps.
    pub async fn update_status<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<Sensor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensors SET
                status = $2,
                last_seen_at = NOW(),
                last_error = $3,
                error_count = CASE WHEN $2 = 'error' THEN error_count + 1 ELSE 0 END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(last_error)
        .fetch_one(executor)
        .await
    }

    /// Get sensors count by type.
    pub async fn count_by_type<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<SensorTypeCount>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT sensor_type, COUNT(*) as count
            FROM sensors
            WHERE organization_id = $1
            GROUP BY sensor_type
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Get sensors near a location (for auto-correlation).
    pub async fn get_sensors_for_building<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
    ) -> Result<Vec<Sensor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM sensors
            WHERE building_id = $1 AND status = 'active'
            ORDER BY name
            "#,
        )
        .bind(building_id)
        .fetch_all(executor)
        .await
    }
}

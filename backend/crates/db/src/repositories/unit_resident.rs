//! Unit resident repository (Epic 3, Story 3.3).

use crate::models::unit_resident::{
    CreateUnitResident, MyUnitRow, UnitResident, UnitResidentSummary, UnitResidentWithUser,
    UpdateUnitResident,
};
use crate::DbPool;
use chrono::NaiveDate;
use sqlx::Error as SqlxError;
use uuid::Uuid;

// #859: explicit column list for `UnitResident`, casting the `resident_type`
// Postgres ENUM to text so sqlx can decode it into the `String` field.
const UNIT_RESIDENT_COLUMNS: &str = "id, unit_id, user_id, resident_type::text AS resident_type, \
     is_primary, start_date, end_date, receives_notifications, receives_mail, notes, \
     created_at, updated_at, created_by";

/// Repository for unit resident operations.
#[derive(Clone)]
pub struct UnitResidentRepository {
    pool: DbPool,
}

impl UnitResidentRepository {
    /// Create a new UnitResidentRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Add a resident to a unit.
    pub async fn create(
        &self,
        data: CreateUnitResident,
        created_by: Uuid,
    ) -> Result<UnitResident, SqlxError> {
        let start_date = data
            .start_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        let resident = sqlx::query_as::<_, UnitResident>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO unit_residents (
                unit_id, user_id, resident_type, is_primary,
                start_date, receives_notifications, receives_mail, notes, created_by
            )
            VALUES ($1, $2, $3::resident_type, $4, $5, $6, $7, $8, $9)
            RETURNING {UNIT_RESIDENT_COLUMNS}
            "#
        )))
        .bind(data.unit_id)
        .bind(data.user_id)
        .bind(&data.resident_type)
        .bind(data.is_primary)
        .bind(start_date)
        .bind(data.receives_notifications)
        .bind(data.receives_mail)
        .bind(&data.notes)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(resident)
    }

    /// Find resident by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<UnitResident>, SqlxError> {
        let resident = sqlx::query_as::<_, UnitResident>(sqlx::AssertSqlSafe(format!(
            "SELECT {UNIT_RESIDENT_COLUMNS} FROM unit_residents WHERE id = $1"
        )))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(resident)
    }

    /// Find all active residents for a unit.
    pub async fn find_by_unit(
        &self,
        unit_id: Uuid,
    ) -> Result<Vec<UnitResidentWithUser>, SqlxError> {
        let residents = sqlx::query_as::<_, UnitResidentWithUser>(
            r#"
            SELECT
                ur.id,
                ur.unit_id,
                ur.user_id,
                u.name as user_name,
                u.email as user_email,
                ur.resident_type::text AS resident_type,
                ur.is_primary,
                ur.start_date,
                ur.end_date
            FROM unit_residents ur
            INNER JOIN users u ON u.id = ur.user_id
            WHERE ur.unit_id = $1
              AND ur.end_date IS NULL
            ORDER BY ur.is_primary DESC, ur.start_date
            "#,
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(residents)
    }

    /// Find all residents for a unit including historical.
    pub async fn find_by_unit_all(
        &self,
        unit_id: Uuid,
    ) -> Result<Vec<UnitResidentWithUser>, SqlxError> {
        let residents = sqlx::query_as::<_, UnitResidentWithUser>(
            r#"
            SELECT
                ur.id,
                ur.unit_id,
                ur.user_id,
                u.name as user_name,
                u.email as user_email,
                ur.resident_type::text AS resident_type,
                ur.is_primary,
                ur.start_date,
                ur.end_date
            FROM unit_residents ur
            INNER JOIN users u ON u.id = ur.user_id
            WHERE ur.unit_id = $1
            ORDER BY ur.end_date NULLS FIRST, ur.start_date DESC
            "#,
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(residents)
    }

    /// Find units for a user.
    pub async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<UnitResidentSummary>, SqlxError> {
        let residents = sqlx::query_as::<_, UnitResidentSummary>(
            r#"
            SELECT
                id, unit_id, user_id, resident_type::text AS resident_type, is_primary,
                (end_date IS NULL) as is_active
            FROM unit_residents
            WHERE user_id = $1
              AND end_date IS NULL
            ORDER BY is_primary DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(residents)
    }

    /// Resolve the authenticated user's active units for the resident-facing
    /// "My Unit" view (Story 3.6).
    ///
    /// Returns one row per active (`end_date IS NULL`) association the user has,
    /// joined to the unit and its building's address. Filtered strictly by
    /// `user_id`, so a caller can only ever see units they are a resident of —
    /// and the projection deliberately omits other residents/owners and the
    /// manager-internal unit `notes`, so no other tenant's PII is exposed.
    pub async fn find_my_units(&self, user_id: Uuid) -> Result<Vec<MyUnitRow>, SqlxError> {
        let rows = sqlx::query_as::<_, MyUnitRow>(
            r#"
            SELECT
                ur.id                AS resident_id,
                ur.resident_type::text AS resident_type,
                ur.is_primary,
                ur.start_date,
                ur.end_date,
                ur.receives_notifications,
                ur.receives_mail,
                u.id                 AS unit_id,
                u.building_id,
                u.entrance,
                u.designation,
                u.floor,
                u.unit_type,
                u.size_sqm,
                u.rooms,
                u.ownership_share,
                u.occupancy_status,
                u.description,
                u.status             AS unit_status,
                b.name               AS building_name,
                b.street             AS building_street,
                b.city               AS building_city,
                b.postal_code        AS building_postal_code
            FROM unit_residents ur
            INNER JOIN units u ON u.id = ur.unit_id
            INNER JOIN buildings b ON b.id = u.building_id
            WHERE ur.user_id = $1
              AND ur.end_date IS NULL
            ORDER BY ur.is_primary DESC, b.street, u.designation
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Update a resident.
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateUnitResident,
    ) -> Result<Option<UnitResident>, SqlxError> {
        let resident = sqlx::query_as::<_, UnitResident>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE unit_residents
            SET
                resident_type = COALESCE($2::resident_type, resident_type),
                is_primary = COALESCE($3, is_primary),
                end_date = COALESCE($4, end_date),
                receives_notifications = COALESCE($5, receives_notifications),
                receives_mail = COALESCE($6, receives_mail),
                notes = COALESCE($7, notes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {UNIT_RESIDENT_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(&data.resident_type)
        .bind(data.is_primary)
        .bind(data.end_date)
        .bind(data.receives_notifications)
        .bind(data.receives_mail)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await?;

        Ok(resident)
    }

    /// End a residency.
    pub async fn end_residency(
        &self,
        id: Uuid,
        end_date: NaiveDate,
    ) -> Result<Option<UnitResident>, SqlxError> {
        let resident = sqlx::query_as::<_, UnitResident>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE unit_residents
            SET end_date = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING {UNIT_RESIDENT_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(end_date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(resident)
    }

    /// Delete a resident (hard delete for cleanup).
    pub async fn delete(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM unit_residents WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Count active residents for a unit.
    pub async fn count_active(&self, unit_id: Uuid) -> Result<i64, SqlxError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM unit_residents
            WHERE unit_id = $1 AND end_date IS NULL
            "#,
        )
        .bind(unit_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}

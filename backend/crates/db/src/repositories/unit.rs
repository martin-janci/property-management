//! Unit repository (Epic 2B, Story 2B.4-5).
//!
//! # RLS Integration
//!
//! This repository supports two usage patterns:
//!
//! 1. **RLS-aware** (recommended): Use methods with `_rls` suffix that accept an executor
//!    with RLS context already set (e.g., from `RlsConnection`).
//!
//! 2. **Legacy**: Use methods without suffix that use the internal pool. These do NOT
//!    enforce RLS and should be migrated to the RLS-aware pattern.
//!
//! ## Example
//!
//! ```rust,ignore
//! async fn create_unit(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateUnitRequest>,
//! ) -> Result<Json<Unit>> {
//!     let unit = state.unit_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(unit))
//! }
//! ```

use crate::models::unit::{
    AssignUnitOwner, CreateUnit, Unit, UnitOwner, UnitOwnerInfo, UnitSummary, UnitWithOwners,
    UpdateUnit,
};
use crate::DbPool;
use rust_decimal::Decimal;
use sqlx::{postgres::PgConnection, Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for unit operations.
#[derive(Clone)]
pub struct UnitRepository {
    pool: DbPool,
}

impl UnitRepository {
    /// Create a new UnitRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    /// Create a new unit with RLS context (UC-15.4).
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_rls<'e, E>(&self, executor: E, data: CreateUnit) -> Result<Unit, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let unit = sqlx::query_as::<_, Unit>(
            r#"
            INSERT INTO units (
                building_id, entrance, designation, floor,
                unit_type, size_sqm, rooms, ownership_share, description
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(data.building_id)
        .bind(&data.entrance)
        .bind(&data.designation)
        .bind(data.floor)
        .bind(&data.unit_type)
        .bind(data.size_sqm)
        .bind(data.rooms)
        .bind(data.ownership_share)
        .bind(&data.description)
        .fetch_one(executor)
        .await?;

        Ok(unit)
    }

    /// Find unit by ID with RLS context (UC-15.5).
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Unit>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let unit = sqlx::query_as::<_, Unit>(
            r#"
            SELECT * FROM units WHERE id = $1 AND status != 'archived'
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(unit)
    }

    /// Update unit with RLS context.
    pub async fn update_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateUnit,
    ) -> Result<Option<Unit>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Build dynamic update query
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 1;

        if data.entrance.is_some() {
            param_idx += 1;
            updates.push(format!("entrance = ${}", param_idx));
        }
        if data.designation.is_some() {
            param_idx += 1;
            updates.push(format!("designation = ${}", param_idx));
        }
        if data.floor.is_some() {
            param_idx += 1;
            updates.push(format!("floor = ${}", param_idx));
        }
        if data.unit_type.is_some() {
            param_idx += 1;
            updates.push(format!("unit_type = ${}", param_idx));
        }
        if data.size_sqm.is_some() {
            param_idx += 1;
            updates.push(format!("size_sqm = ${}", param_idx));
        }
        if data.rooms.is_some() {
            param_idx += 1;
            updates.push(format!("rooms = ${}", param_idx));
        }
        if data.ownership_share.is_some() {
            param_idx += 1;
            updates.push(format!("ownership_share = ${}", param_idx));
        }
        if data.occupancy_status.is_some() {
            param_idx += 1;
            updates.push(format!("occupancy_status = ${}", param_idx));
        }
        if data.description.is_some() {
            param_idx += 1;
            updates.push(format!("description = ${}", param_idx));
        }
        if data.notes.is_some() {
            param_idx += 1;
            updates.push(format!("notes = ${}", param_idx));
        }
        if data.settings.is_some() {
            param_idx += 1;
            updates.push(format!("settings = ${}", param_idx));
        }

        let query = format!(
            "UPDATE units SET {} WHERE id = $1 AND status != 'archived' RETURNING *",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, Unit>(sqlx::query::AssertSqlSafe(&query)).bind(id);

        if let Some(entrance) = &data.entrance {
            q = q.bind(entrance);
        }
        if let Some(designation) = &data.designation {
            q = q.bind(designation);
        }
        if let Some(floor) = &data.floor {
            q = q.bind(floor);
        }
        if let Some(unit_type) = &data.unit_type {
            q = q.bind(unit_type);
        }
        if let Some(size_sqm) = &data.size_sqm {
            q = q.bind(size_sqm);
        }
        if let Some(rooms) = &data.rooms {
            q = q.bind(rooms);
        }
        if let Some(ownership_share) = &data.ownership_share {
            q = q.bind(ownership_share);
        }
        if let Some(occupancy_status) = &data.occupancy_status {
            q = q.bind(occupancy_status);
        }
        if let Some(description) = &data.description {
            q = q.bind(description);
        }
        if let Some(notes) = &data.notes {
            q = q.bind(notes);
        }
        if let Some(settings) = &data.settings {
            q = q.bind(settings);
        }

        let unit = q.fetch_optional(executor).await?;
        Ok(unit)
    }

    /// Archive unit with RLS context (soft delete).
    pub async fn archive_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<Option<Unit>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let unit = sqlx::query_as::<_, Unit>(
            r#"
            UPDATE units
            SET status = 'archived', updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(unit)
    }

    /// Get owners for a unit with RLS context.
    pub async fn get_owners_rls<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
    ) -> Result<Vec<UnitOwnerInfo>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let owners = sqlx::query_as::<_, UnitOwnerInfo>(
            r#"
            SELECT
                uo.user_id,
                u.name as user_name,
                u.email as user_email,
                uo.ownership_percentage,
                uo.is_primary
            FROM unit_owners uo
            INNER JOIN users u ON u.id = uo.user_id
            WHERE uo.unit_id = $1
              AND uo.status = 'active'
              AND (uo.valid_until IS NULL OR uo.valid_until > CURRENT_DATE)
            ORDER BY uo.is_primary DESC, uo.ownership_percentage DESC
            "#,
        )
        .bind(unit_id)
        .fetch_all(executor)
        .await?;

        Ok(owners)
    }

    /// List units for a building with RLS context.
    ///
    /// Returns a simplified Vec without count - use separate count query if needed.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_building_rls<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
        offset: i64,
        limit: i64,
        include_archived: bool,
        unit_type_filter: Option<&str>,
        floor_filter: Option<i32>,
    ) -> Result<Vec<UnitSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let mut conditions = vec!["building_id = $1".to_string()];

        if !include_archived {
            conditions.push("status = 'active'".to_string());
        }

        if unit_type_filter.is_some() {
            conditions.push("unit_type = $4".to_string());
        }

        if floor_filter.is_some() {
            let idx = if unit_type_filter.is_some() { 5 } else { 4 };
            conditions.push(format!("floor = ${}", idx));
        }

        let where_clause = conditions.join(" AND ");

        let data_query = format!(
            r#"
            SELECT id, building_id, designation, floor, unit_type, occupancy_status, status
            FROM units
            WHERE {}
            ORDER BY floor, designation
            LIMIT $2 OFFSET $3
            "#,
            where_clause
        );

        let mut data_q = sqlx::query_as::<_, UnitSummary>(sqlx::query::AssertSqlSafe(&data_query))
            .bind(building_id)
            .bind(limit)
            .bind(offset);
        if let Some(ut) = unit_type_filter {
            data_q = data_q.bind(ut);
        }
        if let Some(f) = floor_filter {
            data_q = data_q.bind(f);
        }
        let units = data_q.fetch_all(executor).await?;

        Ok(units)
    }

    /// List units for a building with count and RLS context.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_building_with_count_rls(
        &self,
        conn: &mut PgConnection,
        building_id: Uuid,
        offset: i64,
        limit: i64,
        include_archived: bool,
        unit_type_filter: Option<&str>,
        floor_filter: Option<i32>,
    ) -> Result<(Vec<UnitSummary>, i64), SqlxError> {
        let mut conditions = vec!["building_id = $1".to_string()];

        if !include_archived {
            conditions.push("status = 'active'".to_string());
        }

        if unit_type_filter.is_some() {
            conditions.push("unit_type = $2".to_string());
        }

        if floor_filter.is_some() {
            let idx = if unit_type_filter.is_some() { 3 } else { 2 };
            conditions.push(format!("floor = ${}", idx));
        }

        let where_clause = conditions.join(" AND ");

        // Get count first
        let count_query = format!("SELECT COUNT(*) FROM units WHERE {}", where_clause);

        let mut count_q = sqlx::query_scalar::<_, i64>(&count_query).bind(building_id);
        if let Some(ut) = unit_type_filter {
            count_q = count_q.bind(ut);
        }
        if let Some(f) = floor_filter {
            count_q = count_q.bind(f);
        }
        let total = count_q.fetch_one(&mut *conn).await?;

        // Get data - need to rebuild conditions with offset/limit params
        let mut data_conditions = vec!["building_id = $1".to_string()];

        if !include_archived {
            data_conditions.push("status = 'active'".to_string());
        }

        if unit_type_filter.is_some() {
            data_conditions.push("unit_type = $4".to_string());
        }

        if floor_filter.is_some() {
            let idx = if unit_type_filter.is_some() { 5 } else { 4 };
            data_conditions.push(format!("floor = ${}", idx));
        }

        let data_where_clause = data_conditions.join(" AND ");

        let data_query = format!(
            r#"
            SELECT id, building_id, designation, floor, unit_type, occupancy_status, status
            FROM units
            WHERE {}
            ORDER BY floor, designation
            LIMIT $2 OFFSET $3
            "#,
            data_where_clause
        );

        let mut data_q = sqlx::query_as::<_, UnitSummary>(sqlx::query::AssertSqlSafe(&data_query))
            .bind(building_id)
            .bind(limit)
            .bind(offset);
        if let Some(ut) = unit_type_filter {
            data_q = data_q.bind(ut);
        }
        if let Some(f) = floor_filter {
            data_q = data_q.bind(f);
        }
        let units = data_q.fetch_all(&mut *conn).await?;

        Ok((units, total))
    }

    /// Check if designation exists in building with RLS context.
    pub async fn designation_exists_rls<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
        designation: &str,
        exclude_unit_id: Option<Uuid>,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count: i64 = if let Some(exclude_id) = exclude_unit_id {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM units
                WHERE building_id = $1 AND LOWER(designation) = LOWER($2) AND id != $3
                "#,
            )
            .bind(building_id)
            .bind(designation)
            .bind(exclude_id)
            .fetch_one(executor)
            .await?
        } else {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM units
                WHERE building_id = $1 AND LOWER(designation) = LOWER($2)
                "#,
            )
            .bind(building_id)
            .bind(designation)
            .fetch_one(executor)
            .await?
        };

        Ok(count > 0)
    }

    /// Check if unit belongs to building with RLS context.
    pub async fn belongs_to_building_rls<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
        building_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM units
            WHERE id = $1 AND building_id = $2
            "#,
        )
        .bind(unit_id)
        .bind(building_id)
        .fetch_one(executor)
        .await?;

        Ok(count > 0)
    }

    /// Get total ownership percentage for a unit with RLS context.
    pub async fn get_total_ownership_rls<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
    ) -> Result<Decimal, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let total: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(ownership_percentage), 0)
            FROM unit_owners
            WHERE unit_id = $1
              AND status = 'active'
              AND (valid_until IS NULL OR valid_until > CURRENT_DATE)
            "#,
        )
        .bind(unit_id)
        .fetch_one(executor)
        .await?;

        Ok(total.unwrap_or_default())
    }

    /// Assign owner to unit with RLS context (UC-15.6).
    pub async fn assign_owner_rls<'e, E>(
        &self,
        executor: E,
        data: AssignUnitOwner,
    ) -> Result<UnitOwner, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let valid_from = data
            .valid_from
            .unwrap_or_else(|| chrono::Utc::now().naive_utc().date());

        let owner = sqlx::query_as::<_, UnitOwner>(
            r#"
            INSERT INTO unit_owners (unit_id, user_id, ownership_percentage, is_primary, valid_from)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.unit_id)
        .bind(data.user_id)
        .bind(data.ownership_percentage)
        .bind(data.is_primary)
        .bind(valid_from)
        .fetch_one(executor)
        .await?;

        Ok(owner)
    }

    /// Update owner assignment with RLS context.
    pub async fn update_owner_rls<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
        user_id: Uuid,
        ownership_percentage: Option<Decimal>,
        is_primary: Option<bool>,
    ) -> Result<Option<UnitOwner>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 2;

        if ownership_percentage.is_some() {
            param_idx += 1;
            updates.push(format!("ownership_percentage = ${}", param_idx));
        }
        if is_primary.is_some() {
            param_idx += 1;
            updates.push(format!("is_primary = ${}", param_idx));
        }

        let query = format!(
            "UPDATE unit_owners SET {} WHERE unit_id = $1 AND user_id = $2 AND status = 'active' RETURNING *",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, UnitOwner>(sqlx::query::AssertSqlSafe(&query))
            .bind(unit_id)
            .bind(user_id);

        if let Some(pct) = ownership_percentage {
            q = q.bind(pct);
        }
        if let Some(primary) = is_primary {
            q = q.bind(primary);
        }

        let owner = q.fetch_optional(executor).await?;
        Ok(owner)
    }

    /// Remove owner from unit with RLS context.
    pub async fn remove_owner_rls<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE unit_owners
            SET status = 'inactive', valid_until = CURRENT_DATE, updated_at = NOW()
            WHERE unit_id = $1 AND user_id = $2 AND status = 'active'
            "#,
        )
        .bind(unit_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Restore archived unit with RLS context.
    pub async fn restore_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<Option<Unit>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let unit = sqlx::query_as::<_, Unit>(
            r#"
            UPDATE units
            SET status = 'active', updated_at = NOW()
            WHERE id = $1 AND status = 'archived'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(unit)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    /// Create a new unit (UC-15.4).
    ///
    /// **Deprecated**: Use `create_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.275", note = "Use create_rls with RlsConnection instead")]
    pub async fn create(&self, data: CreateUnit) -> Result<Unit, SqlxError> {
        self.create_rls(&self.pool, data).await
    }

    /// Find unit by ID (UC-15.5).
    ///
    /// **Deprecated**: Use `find_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.275",
        note = "Use find_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Unit>, SqlxError> {
        self.find_by_id_rls(&self.pool, id).await
    }

    /// Find unit by ID with owners.
    pub async fn find_by_id_with_owners(
        &self,
        id: Uuid,
    ) -> Result<Option<UnitWithOwners>, SqlxError> {
        #[allow(deprecated)]
        let unit = self.find_by_id(id).await?;

        match unit {
            Some(u) => {
                #[allow(deprecated)]
                let owners = self.get_owners(id).await?;
                Ok(Some(UnitWithOwners { unit: u, owners }))
            }
            None => Ok(None),
        }
    }

    /// Get owners for a unit.
    ///
    /// **Deprecated**: Use `get_owners_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.275",
        note = "Use get_owners_rls with RlsConnection instead"
    )]
    pub async fn get_owners(&self, unit_id: Uuid) -> Result<Vec<UnitOwnerInfo>, SqlxError> {
        self.get_owners_rls(&self.pool, unit_id).await
    }

    /// Update unit.
    ///
    /// **Deprecated**: Use `update_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.275", note = "Use update_rls with RlsConnection instead")]
    pub async fn update(&self, id: Uuid, data: UpdateUnit) -> Result<Option<Unit>, SqlxError> {
        self.update_rls(&self.pool, id, data).await
    }

    /// Archive unit (soft delete).
    ///
    /// **Deprecated**: Use `archive_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.275", note = "Use archive_rls with RlsConnection instead")]
    pub async fn archive(&self, id: Uuid) -> Result<Option<Unit>, SqlxError> {
        self.archive_rls(&self.pool, id).await
    }

    /// Restore archived unit.
    pub async fn restore(&self, id: Uuid) -> Result<Option<Unit>, SqlxError> {
        let unit = sqlx::query_as::<_, Unit>(
            r#"
            UPDATE units
            SET status = 'active', updated_at = NOW()
            WHERE id = $1 AND status = 'archived'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(unit)
    }

    /// List units for a building.
    ///
    /// **Deprecated**: Use `list_by_building_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.275",
        note = "Use list_by_building_rls with RlsConnection instead"
    )]
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_building(
        &self,
        building_id: Uuid,
        offset: i64,
        limit: i64,
        include_archived: bool,
        unit_type_filter: Option<&str>,
        floor_filter: Option<i32>,
    ) -> Result<(Vec<UnitSummary>, i64), SqlxError> {
        let mut conditions = vec!["building_id = $1".to_string()];

        if !include_archived {
            conditions.push("status = 'active'".to_string());
        }

        if unit_type_filter.is_some() {
            conditions.push("unit_type = $4".to_string());
        }

        if floor_filter.is_some() {
            let idx = if unit_type_filter.is_some() { 5 } else { 4 };
            conditions.push(format!("floor = ${}", idx));
        }

        let where_clause = conditions.join(" AND ");

        let count_query = format!("SELECT COUNT(*) FROM units WHERE {}", where_clause);

        // Execute count query
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_query).bind(building_id);
        if let Some(ut) = unit_type_filter {
            count_q = count_q.bind(ut);
        }
        if let Some(f) = floor_filter {
            count_q = count_q.bind(f);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // Execute data query via RLS method
        let units = self
            .list_by_building_rls(
                &self.pool,
                building_id,
                offset,
                limit,
                include_archived,
                unit_type_filter,
                floor_filter,
            )
            .await?;

        Ok((units, total))
    }

    /// Assign owner to unit (UC-15.6).
    pub async fn assign_owner(&self, data: AssignUnitOwner) -> Result<UnitOwner, SqlxError> {
        let valid_from = data
            .valid_from
            .unwrap_or_else(|| chrono::Utc::now().naive_utc().date());

        let owner = sqlx::query_as::<_, UnitOwner>(
            r#"
            INSERT INTO unit_owners (unit_id, user_id, ownership_percentage, is_primary, valid_from)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.unit_id)
        .bind(data.user_id)
        .bind(data.ownership_percentage)
        .bind(data.is_primary)
        .bind(valid_from)
        .fetch_one(&self.pool)
        .await?;

        Ok(owner)
    }

    /// Remove owner from unit.
    pub async fn remove_owner(&self, unit_id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE unit_owners
            SET status = 'inactive', valid_until = CURRENT_DATE, updated_at = NOW()
            WHERE unit_id = $1 AND user_id = $2 AND status = 'active'
            "#,
        )
        .bind(unit_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update owner assignment.
    pub async fn update_owner(
        &self,
        unit_id: Uuid,
        user_id: Uuid,
        ownership_percentage: Option<Decimal>,
        is_primary: Option<bool>,
    ) -> Result<Option<UnitOwner>, SqlxError> {
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 2;

        if ownership_percentage.is_some() {
            param_idx += 1;
            updates.push(format!("ownership_percentage = ${}", param_idx));
        }
        if is_primary.is_some() {
            param_idx += 1;
            updates.push(format!("is_primary = ${}", param_idx));
        }

        let query = format!(
            "UPDATE unit_owners SET {} WHERE unit_id = $1 AND user_id = $2 AND status = 'active' RETURNING *",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, UnitOwner>(sqlx::query::AssertSqlSafe(&query))
            .bind(unit_id)
            .bind(user_id);

        if let Some(pct) = ownership_percentage {
            q = q.bind(pct);
        }
        if let Some(primary) = is_primary {
            q = q.bind(primary);
        }

        let owner = q.fetch_optional(&self.pool).await?;
        Ok(owner)
    }

    /// Get total ownership percentage for a unit.
    pub async fn get_total_ownership(&self, unit_id: Uuid) -> Result<Decimal, SqlxError> {
        let total: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(ownership_percentage), 0)
            FROM unit_owners
            WHERE unit_id = $1
              AND status = 'active'
              AND (valid_until IS NULL OR valid_until > CURRENT_DATE)
            "#,
        )
        .bind(unit_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(total.unwrap_or_default())
    }

    /// Check if unit belongs to building.
    pub async fn belongs_to_building(
        &self,
        unit_id: Uuid,
        building_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM units
            WHERE id = $1 AND building_id = $2
            "#,
        )
        .bind(unit_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get units owned by a user.
    pub async fn get_user_units(&self, user_id: Uuid) -> Result<Vec<UnitSummary>, SqlxError> {
        let units = sqlx::query_as::<_, UnitSummary>(
            r#"
            SELECT u.id, u.building_id, u.designation, u.floor, u.unit_type, u.occupancy_status, u.status
            FROM units u
            INNER JOIN unit_owners uo ON uo.unit_id = u.id
            WHERE uo.user_id = $1
              AND uo.status = 'active'
              AND u.status = 'active'
              AND (uo.valid_until IS NULL OR uo.valid_until > CURRENT_DATE)
            ORDER BY u.designation
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(units)
    }

    /// Check if designation exists in building.
    pub async fn designation_exists(
        &self,
        building_id: Uuid,
        designation: &str,
        exclude_unit_id: Option<Uuid>,
    ) -> Result<bool, SqlxError> {
        let query = if exclude_unit_id.is_some() {
            r#"
            SELECT COUNT(*) FROM units
            WHERE building_id = $1 AND LOWER(designation) = LOWER($2) AND id != $3
            "#
        } else {
            r#"
            SELECT COUNT(*) FROM units
            WHERE building_id = $1 AND LOWER(designation) = LOWER($2)
            "#
        };

        let mut q = sqlx::query_scalar::<_, i64>(query)
            .bind(building_id)
            .bind(designation);

        if let Some(exclude_id) = exclude_unit_id {
            q = q.bind(exclude_id);
        }

        let count = q.fetch_one(&self.pool).await?;
        Ok(count > 0)
    }

    /// Count units by organization (Story 88.5: Report Export Fallback).
    ///
    /// Counts all active units in all buildings belonging to an organization.
    /// Used for estimating report size to decide sync vs async processing.
    pub async fn count_by_organization(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM units u
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1 AND u.status = 'active'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}

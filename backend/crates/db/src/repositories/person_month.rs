//! Person month repository (Epic 3, Story 3.5).

use crate::models::person_month::{
    BuildingPersonMonthSummary, BulkPersonMonthEntry, CreatePersonMonth, MonthlyCount, PersonMonth,
    PersonMonthWithUnit, UpdatePersonMonth, YearlyPersonMonthSummary,
};
use crate::DbPool;
use sqlx::{postgres::PgConnection, Error as SqlxError};
use uuid::Uuid;

/// Explicit column list for `PersonMonth` rows.
///
/// #859: sqlx 0.9 cannot decode a Postgres ENUM column into a Rust `String`,
/// so the `source` (`person_month_source`) enum is cast to text. The rest map
/// 1:1 to the struct fields. Used instead of `SELECT *` / `RETURNING *`.
const PERSON_MONTH_COLUMNS: &str = "id, unit_id, year, month, count, source::text AS source, \
     notes, created_at, updated_at, created_by, updated_by";

/// Repository for person month operations.
#[derive(Clone)]
pub struct PersonMonthRepository {
    pool: DbPool,
}

impl PersonMonthRepository {
    /// Create a new PersonMonthRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create or update a person month entry.
    ///
    /// Runs on the caller-supplied connection so the request's RLS context
    /// (`app.current_user_id` / `app.current_org_id`) is in scope — the
    /// `person_months_insert_manager` policy requires it. Pass
    /// `&mut **rls.conn()` from the handler.
    pub async fn upsert(
        &self,
        conn: &mut PgConnection,
        data: CreatePersonMonth,
        user_id: Uuid,
    ) -> Result<PersonMonth, SqlxError> {
        let entry = sqlx::query_as::<_, PersonMonth>(sqlx::AssertSqlSafe(format!(r#"
            INSERT INTO person_months (unit_id, year, month, count, source, notes, created_by, updated_by)
            VALUES ($1, $2, $3, $4, $5::person_month_source, $6, $7, $7)
            ON CONFLICT (unit_id, year, month)
            DO UPDATE SET
                count = EXCLUDED.count,
                source = EXCLUDED.source,
                notes = EXCLUDED.notes,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            RETURNING {PERSON_MONTH_COLUMNS}
            "#)))
        .bind(data.unit_id)
        .bind(data.year)
        .bind(data.month)
        .bind(data.count)
        .bind(&data.source)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(conn)
        .await?;

        Ok(entry)
    }

    /// Find person month by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<PersonMonth>, SqlxError> {
        let entry = sqlx::query_as::<_, PersonMonth>(sqlx::AssertSqlSafe(format!(
            r#"SELECT {PERSON_MONTH_COLUMNS} FROM person_months WHERE id = $1"#
        )))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Find person month for a unit and period.
    pub async fn find_by_unit_period(
        &self,
        unit_id: Uuid,
        year: i32,
        month: i32,
    ) -> Result<Option<PersonMonth>, SqlxError> {
        let entry = sqlx::query_as::<_, PersonMonth>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {PERSON_MONTH_COLUMNS} FROM person_months
            WHERE unit_id = $1 AND year = $2 AND month = $3
            "#
        )))
        .bind(unit_id)
        .bind(year)
        .bind(month)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Find all person months for a unit in a year.
    pub async fn find_by_unit_year(
        &self,
        unit_id: Uuid,
        year: i32,
    ) -> Result<Vec<PersonMonth>, SqlxError> {
        let entries = sqlx::query_as::<_, PersonMonth>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {PERSON_MONTH_COLUMNS} FROM person_months
            WHERE unit_id = $1 AND year = $2
            ORDER BY month
            "#
        )))
        .bind(unit_id)
        .bind(year)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Get yearly summary for a unit.
    pub async fn get_yearly_summary(
        &self,
        unit_id: Uuid,
        year: i32,
    ) -> Result<YearlyPersonMonthSummary, SqlxError> {
        let entries = self.find_by_unit_year(unit_id, year).await?;

        let mut months: Vec<MonthlyCount> = (1..=12)
            .map(|m| MonthlyCount {
                month: m,
                count: 0,
                source: "calculated".to_string(),
            })
            .collect();

        let mut total = 0;

        for entry in entries {
            if let Some(month_entry) = months.get_mut((entry.month - 1) as usize) {
                month_entry.count = entry.count;
                month_entry.source = entry.source;
                total += entry.count;
            }
        }

        Ok(YearlyPersonMonthSummary {
            unit_id,
            year,
            months,
            total,
        })
    }

    /// Find person months for a building in a period.
    pub async fn find_by_building_period(
        &self,
        building_id: Uuid,
        year: i32,
        month: i32,
    ) -> Result<Vec<PersonMonthWithUnit>, SqlxError> {
        // #859: cast the `source` enum to text so sqlx 0.9 can decode it as String.
        let entries = sqlx::query_as::<_, PersonMonthWithUnit>(
            r#"
            SELECT
                pm.id,
                pm.unit_id,
                u.designation as unit_designation,
                pm.year,
                pm.month,
                pm.count,
                pm.source::text AS source
            FROM person_months pm
            INNER JOIN units u ON u.id = pm.unit_id
            WHERE u.building_id = $1 AND pm.year = $2 AND pm.month = $3
            ORDER BY u.floor, u.designation
            "#,
        )
        .bind(building_id)
        .bind(year)
        .bind(month)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Get building-level summary.
    pub async fn get_building_summary(
        &self,
        building_id: Uuid,
        year: i32,
        month: i32,
    ) -> Result<Option<BuildingPersonMonthSummary>, SqlxError> {
        let summary = sqlx::query_as::<_, BuildingPersonMonthSummary>(
            r#"
            SELECT
                $1::UUID as building_id,
                $2::INTEGER as year,
                $3::INTEGER as month,
                COALESCE(SUM(pm.count), 0) as total_count,
                COUNT(pm.id) as unit_count
            FROM units u
            LEFT JOIN person_months pm ON pm.unit_id = u.id
                AND pm.year = $2 AND pm.month = $3
            WHERE u.building_id = $1
            "#,
        )
        .bind(building_id)
        .bind(year)
        .bind(month)
        .fetch_optional(&self.pool)
        .await?;

        Ok(summary)
    }

    /// Bulk upsert person months for a building.
    ///
    /// Runs on the caller-supplied RLS connection (see [`Self::upsert`]); the
    /// connection is reborrowed per iteration. Pass `&mut **rls.conn()`.
    pub async fn bulk_upsert(
        &self,
        conn: &mut PgConnection,
        year: i32,
        month: i32,
        entries: Vec<BulkPersonMonthEntry>,
        user_id: Uuid,
    ) -> Result<Vec<PersonMonth>, SqlxError> {
        let mut results = Vec::with_capacity(entries.len());

        for entry in entries {
            let pm = self
                .upsert(
                    &mut *conn,
                    CreatePersonMonth {
                        unit_id: entry.unit_id,
                        year,
                        month,
                        count: entry.count,
                        source: "manual".to_string(),
                        notes: None,
                    },
                    user_id,
                )
                .await?;
            results.push(pm);
        }

        Ok(results)
    }

    /// Update a person month.
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdatePersonMonth,
        user_id: Uuid,
    ) -> Result<Option<PersonMonth>, SqlxError> {
        let entry = sqlx::query_as::<_, PersonMonth>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE person_months
            SET
                count = COALESCE($2, count),
                source = COALESCE($3::person_month_source, source),
                notes = COALESCE($4, notes),
                updated_by = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING {PERSON_MONTH_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(data.count)
        .bind(&data.source)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Delete a person month entry.
    pub async fn delete(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM person_months WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Calculate person months from residents for a period.
    pub async fn calculate_from_residents(
        &self,
        unit_id: Uuid,
        year: i32,
        month: i32,
    ) -> Result<i32, SqlxError> {
        let count: (i64,) =
            sqlx::query_as(r#"SELECT count_residents_for_month($1, $2, $3)::bigint"#)
                .bind(unit_id)
                .bind(year)
                .bind(month)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0 as i32)
    }

    // ========================================================================
    // Epic 55: Reporting Analytics
    // ========================================================================

    /// Get occupancy report data (Epic 55, Story 55.3).
    pub async fn get_occupancy_report(
        &self,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
    ) -> Result<crate::models::reports::OccupancyReportData, SqlxError> {
        // Get total units count
        let (total_units,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM units u
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
            "#,
        )
        .bind(organization_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        // Get occupied units (units with person_months in the period)
        let (occupied_units,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT pm.unit_id) FROM person_months pm
            JOIN units u ON pm.unit_id = u.id
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
              AND (pm.year > EXTRACT(YEAR FROM $3::date) OR
                   (pm.year = EXTRACT(YEAR FROM $3::date) AND pm.month >= EXTRACT(MONTH FROM $3::date)))
              AND (pm.year < EXTRACT(YEAR FROM $4::date) OR
                   (pm.year = EXTRACT(YEAR FROM $4::date) AND pm.month <= EXTRACT(MONTH FROM $4::date)))
              AND pm.count > 0
            "#,
        )
        .bind(organization_id)
        .bind(building_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_one(&self.pool)
        .await?;

        // Get total person months
        let (total_person_months,): (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(pm.count), 0) FROM person_months pm
            JOIN units u ON pm.unit_id = u.id
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
              AND (pm.year > EXTRACT(YEAR FROM $3::date) OR
                   (pm.year = EXTRACT(YEAR FROM $3::date) AND pm.month >= EXTRACT(MONTH FROM $3::date)))
              AND (pm.year < EXTRACT(YEAR FROM $4::date) OR
                   (pm.year = EXTRACT(YEAR FROM $4::date) AND pm.month <= EXTRACT(MONTH FROM $4::date)))
            "#,
        )
        .bind(organization_id)
        .bind(building_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_one(&self.pool)
        .await?;

        // Get monthly totals
        let monthly_totals = sqlx::query_as::<_, crate::models::reports::ReportMonthlyCount>(
            r#"
            SELECT pm.year, pm.month, SUM(pm.count)::int8 as count
            FROM person_months pm
            JOIN units u ON pm.unit_id = u.id
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
              AND (pm.year > EXTRACT(YEAR FROM $3::date) OR
                   (pm.year = EXTRACT(YEAR FROM $3::date) AND pm.month >= EXTRACT(MONTH FROM $3::date)))
              AND (pm.year < EXTRACT(YEAR FROM $4::date) OR
                   (pm.year = EXTRACT(YEAR FROM $4::date) AND pm.month <= EXTRACT(MONTH FROM $4::date)))
            GROUP BY pm.year, pm.month
            ORDER BY pm.year, pm.month
            "#,
        )
        .bind(organization_id)
        .bind(building_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&self.pool)
        .await?;

        let vacant_units = total_units - occupied_units;
        let occupancy_rate = if total_units > 0 {
            (occupied_units as f64 / total_units as f64) * 100.0
        } else {
            0.0
        };
        let average_occupants_per_unit = if occupied_units > 0 {
            total_person_months as f64 / occupied_units as f64
        } else {
            0.0
        };

        Ok(crate::models::reports::OccupancyReportData {
            summary: crate::models::reports::OccupancySummary {
                total_units,
                occupied_units,
                vacant_units,
                occupancy_rate,
                total_person_months,
                average_occupants_per_unit,
            },
            by_unit: vec![], // Could be expanded with per-unit details if needed
            monthly_totals,
        })
    }
}

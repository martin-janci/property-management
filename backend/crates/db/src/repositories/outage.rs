//! Outage repository (UC-12: Utility Outages).
//!
//! # RLS Integration
//!
//! This repository uses RLS-aware methods that accept an executor
//! with RLS context already set (e.g., from `RlsConnection`).
//!
//! ## Example
//!
//! ```rust,ignore
//! async fn create_outage(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateOutageRequest>,
//! ) -> Result<Json<Outage>> {
//!     let outage = state.outage_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(outage))
//! }
//! ```

use crate::models::outage::{
    CommodityCount, CreateOutage, CreateOutageNotification, Outage, OutageDashboard,
    OutageListQuery, OutageNotification, OutageStatistics, OutageSummary, OutageWithDetails,
    UpdateOutage,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::{Error as SqlxError, Executor, FromRow, Postgres, Row};
use uuid::Uuid;

/// Row struct for outage with details query.
#[derive(Debug, FromRow)]
struct OutageDetailsRow {
    // Outage fields
    pub id: Uuid,
    pub organization_id: Uuid,
    pub created_by: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub commodity: String,
    pub severity: String,
    pub status: String,
    pub building_ids: serde_json::Value,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub external_reference: Option<String>,
    pub supplier_name: Option<String>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined fields
    pub creator_name: String,
    pub notification_count: i64,
    pub read_count: i64,
}

/// Repository for outage operations.
#[derive(Clone)]
pub struct OutageRepository {
    pool: DbPool,
}

impl OutageRepository {
    /// Create a new OutageRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods
    // ========================================================================

    // ------------------------------------------------------------------------
    // Outage CRUD
    // ------------------------------------------------------------------------

    /// Create a new outage with RLS context.
    pub async fn create_rls<'e, E>(
        &self,
        executor: E,
        data: CreateOutage,
    ) -> Result<Outage, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let building_ids_json = serde_json::to_value(&data.building_ids).unwrap_or_default();

        let outage = sqlx::query_as::<_, Outage>(
            r#"
            INSERT INTO outages (
                organization_id, created_by, title, description,
                commodity, severity, status, building_ids,
                scheduled_start, scheduled_end,
                external_reference, supplier_name
            )
            VALUES ($1, $2, $3, $4, $5::outage_commodity, $6::outage_severity, 'planned'::outage_status, $7, $8, $9, $10, $11)
            RETURNING
                id, organization_id, created_by, title, description,
                commodity::text as commodity, severity::text as severity, status::text as status,
                building_ids, scheduled_start, scheduled_end,
                actual_start, actual_end, external_reference, supplier_name,
                resolution_notes, created_at, updated_at
            "#,
        )
        .bind(data.organization_id)
        .bind(data.created_by)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.commodity)
        .bind(&data.severity)
        .bind(&building_ids_json)
        .bind(data.scheduled_start)
        .bind(data.scheduled_end)
        .bind(&data.external_reference)
        .bind(&data.supplier_name)
        .fetch_one(executor)
        .await?;

        Ok(outage)
    }

    /// Find outage by ID with RLS context.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Outage>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let outage = sqlx::query_as::<_, Outage>(
            r#"
            SELECT
                id, organization_id, created_by, title, description,
                commodity::text as commodity, severity::text as severity, status::text as status,
                building_ids, scheduled_start, scheduled_end,
                actual_start, actual_end, external_reference, supplier_name,
                resolution_notes, created_at, updated_at
            FROM outages WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(outage)
    }

    /// Find outage with full details with RLS context.
    pub async fn find_by_id_with_details_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<OutageWithDetails>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query_as::<_, OutageDetailsRow>(
            r#"
            SELECT
                o.id, o.organization_id, o.created_by, o.title, o.description,
                o.commodity::text as commodity, o.severity::text as severity, o.status::text as status,
                o.building_ids, o.scheduled_start, o.scheduled_end,
                o.actual_start, o.actual_end, o.external_reference, o.supplier_name,
                o.resolution_notes, o.created_at, o.updated_at,
                u.name as creator_name,
                (SELECT COUNT(*) FROM outage_notifications WHERE outage_id = o.id) as notification_count,
                (SELECT COUNT(*) FROM outage_notifications WHERE outage_id = o.id AND read_at IS NOT NULL) as read_count
            FROM outages o
            JOIN users u ON o.created_by = u.id
            WHERE o.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|row| {
            let outage = Outage {
                id: row.id,
                organization_id: row.organization_id,
                created_by: row.created_by,
                title: row.title,
                description: row.description,
                commodity: row.commodity,
                severity: row.severity,
                status: row.status,
                building_ids: row.building_ids,
                scheduled_start: row.scheduled_start,
                scheduled_end: row.scheduled_end,
                actual_start: row.actual_start,
                actual_end: row.actual_end,
                external_reference: row.external_reference,
                supplier_name: row.supplier_name,
                resolution_notes: row.resolution_notes,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };

            OutageWithDetails {
                outage,
                creator_name: row.creator_name,
                building_names: vec![], // Would need separate query to resolve building names
                notification_count: row.notification_count,
                read_count: row.read_count,
            }
        }))
    }

    /// List outages with filters and pagination.
    pub async fn list_rls<'e, E>(
        &self,
        executor: E,
        query: &OutageListQuery,
    ) -> Result<Vec<OutageSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        // Build dynamic WHERE clause
        let mut conditions = vec!["1=1".to_string()];

        if query.active_only.unwrap_or(false) {
            conditions.push("status IN ('planned', 'ongoing')".to_string());
        }

        // Note: We'd need to bind dynamic parameters differently for a production implementation
        // This is a simplified version

        let sql = format!(
            r#"
            SELECT
                id, title,
                commodity::text as commodity, severity::text as severity, status::text as status,
                scheduled_start, scheduled_end, actual_start, actual_end
            FROM outages
            WHERE {}
            ORDER BY
                CASE status
                    WHEN 'ongoing' THEN 1
                    WHEN 'planned' THEN 2
                    ELSE 3
                END,
                scheduled_start DESC
            LIMIT $1 OFFSET $2
            "#,
            conditions.join(" AND ")
        );

        let outages = sqlx::query_as::<_, OutageSummary>(sqlx::AssertSqlSafe(sql))
            .bind(limit)
            .bind(offset)
            .fetch_all(executor)
            .await?;

        Ok(outages)
    }

    /// Count outages matching query.
    pub async fn count_rls<'e, E>(
        &self,
        executor: E,
        query: &OutageListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let mut conditions = vec!["1=1".to_string()];

        if query.active_only.unwrap_or(false) {
            conditions.push("status IN ('planned', 'ongoing')".to_string());
        }

        let sql = format!(
            "SELECT COUNT(*) as count FROM outages WHERE {}",
            conditions.join(" AND ")
        );

        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_one(executor)
            .await?;
        let count: i64 = row.get("count");

        Ok(count)
    }

    /// Update an outage.
    pub async fn update_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateOutage,
    ) -> Result<Outage, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let building_ids_json = data
            .building_ids
            .as_ref()
            .map(|ids| serde_json::to_value(ids).unwrap_or_default());

        let outage = sqlx::query_as::<_, Outage>(
            r#"
            UPDATE outages SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                commodity = COALESCE($4::outage_commodity, commodity),
                severity = COALESCE($5::outage_severity, severity),
                building_ids = COALESCE($6, building_ids),
                scheduled_start = COALESCE($7, scheduled_start),
                scheduled_end = COALESCE($8, scheduled_end),
                external_reference = COALESCE($9, external_reference),
                supplier_name = COALESCE($10, supplier_name),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, organization_id, created_by, title, description,
                commodity::text as commodity, severity::text as severity, status::text as status,
                building_ids, scheduled_start, scheduled_end,
                actual_start, actual_end, external_reference, supplier_name,
                resolution_notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.commodity)
        .bind(&data.severity)
        .bind(&building_ids_json)
        .bind(data.scheduled_start)
        .bind(data.scheduled_end)
        .bind(&data.external_reference)
        .bind(&data.supplier_name)
        .fetch_one(executor)
        .await?;

        Ok(outage)
    }

    /// Start an outage (change status from planned to ongoing).
    pub async fn start_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        actual_start: Option<DateTime<Utc>>,
    ) -> Result<Outage, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let start_time = actual_start.unwrap_or_else(Utc::now);

        let outage = sqlx::query_as::<_, Outage>(
            r#"
            UPDATE outages SET
                status = 'ongoing'::outage_status,
                actual_start = $2,
                updated_at = NOW()
            WHERE id = $1 AND status = 'planned'::outage_status
            RETURNING
                id, organization_id, created_by, title, description,
                commodity::text as commodity, severity::text as severity, status::text as status,
                building_ids, scheduled_start, scheduled_end,
                actual_start, actual_end, external_reference, supplier_name,
                resolution_notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(start_time)
        .fetch_one(executor)
        .await?;

        Ok(outage)
    }

    /// Resolve an outage.
    pub async fn resolve_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        actual_end: Option<DateTime<Utc>>,
        resolution_notes: Option<String>,
    ) -> Result<Outage, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let end_time = actual_end.unwrap_or_else(Utc::now);

        let outage = sqlx::query_as::<_, Outage>(
            r#"
            UPDATE outages SET
                status = 'resolved'::outage_status,
                actual_end = $2,
                resolution_notes = COALESCE($3, resolution_notes),
                updated_at = NOW()
            WHERE id = $1 AND status IN ('planned'::outage_status, 'ongoing'::outage_status)
            RETURNING
                id, organization_id, created_by, title, description,
                commodity::text as commodity, severity::text as severity, status::text as status,
                building_ids, scheduled_start, scheduled_end,
                actual_start, actual_end, external_reference, supplier_name,
                resolution_notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(end_time)
        .bind(&resolution_notes)
        .fetch_one(executor)
        .await?;

        Ok(outage)
    }

    /// Cancel an outage.
    pub async fn cancel_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        reason: Option<String>,
    ) -> Result<Outage, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let outage = sqlx::query_as::<_, Outage>(
            r#"
            UPDATE outages SET
                status = 'cancelled'::outage_status,
                resolution_notes = COALESCE($2, resolution_notes),
                updated_at = NOW()
            WHERE id = $1 AND status IN ('planned'::outage_status, 'ongoing'::outage_status)
            RETURNING
                id, organization_id, created_by, title, description,
                commodity::text as commodity, severity::text as severity, status::text as status,
                building_ids, scheduled_start, scheduled_end,
                actual_start, actual_end, external_reference, supplier_name,
                resolution_notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&reason)
        .fetch_one(executor)
        .await?;

        Ok(outage)
    }

    /// Delete an outage.
    pub async fn delete_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM outages WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Statistics & Dashboard
    // ------------------------------------------------------------------------

    /// Get outage statistics.
    pub async fn get_statistics_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<OutageStatistics, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let stats = sqlx::query_as::<_, OutageStatistics>(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'planned') as planned,
                COUNT(*) FILTER (WHERE status = 'ongoing') as ongoing,
                COUNT(*) FILTER (WHERE status = 'resolved') as resolved,
                COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled
            FROM outages
            "#,
        )
        .fetch_one(executor)
        .await?;

        Ok(stats)
    }

    /// Get outage dashboard data.
    pub async fn get_dashboard_rls<'e, E>(&self, executor: E) -> Result<OutageDashboard, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Get counts
        let stats = self.get_statistics_rls(&self.pool).await?;

        // Get upcoming outages
        let upcoming = sqlx::query_as::<_, OutageSummary>(
            r#"
            SELECT
                id, title,
                commodity::text as commodity, severity::text as severity, status::text as status,
                scheduled_start, scheduled_end, actual_start, actual_end
            FROM outages
            WHERE status IN ('planned', 'ongoing')
            ORDER BY scheduled_start ASC
            LIMIT 10
            "#,
        )
        .fetch_all(executor)
        .await?;

        // Get counts by commodity
        let by_commodity = sqlx::query_as::<_, CommodityCount>(
            r#"
            SELECT commodity::text as commodity, COUNT(*) as count
            FROM outages
            WHERE status IN ('planned', 'ongoing')
            GROUP BY commodity
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(OutageDashboard {
            active_count: stats.planned + stats.ongoing,
            planned_count: stats.planned,
            ongoing_count: stats.ongoing,
            by_commodity,
            upcoming,
        })
    }

    // ------------------------------------------------------------------------
    // Notifications
    // ------------------------------------------------------------------------

    /// Create notification record for a user.
    pub async fn create_notification_rls<'e, E>(
        &self,
        executor: E,
        data: CreateOutageNotification,
    ) -> Result<OutageNotification, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let notification = sqlx::query_as::<_, OutageNotification>(
            r#"
            INSERT INTO outage_notifications (outage_id, user_id, notification_method)
            VALUES ($1, $2, $3)
            ON CONFLICT (outage_id, user_id, notification_method) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(data.outage_id)
        .bind(data.user_id)
        .bind(&data.notification_method)
        .fetch_one(executor)
        .await?;

        Ok(notification)
    }

    /// Mark outage notification as read.
    pub async fn mark_read_rls<'e, E>(
        &self,
        executor: E,
        outage_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE outage_notifications
            SET read_at = NOW()
            WHERE outage_id = $1 AND user_id = $2 AND read_at IS NULL
            "#,
        )
        .bind(outage_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Get unread outage count for a user.
    pub async fn count_unread_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM outage_notifications
            WHERE user_id = $1 AND read_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        let count: i64 = row.get("count");
        Ok(count)
    }

    /// Find active outages for specific buildings.
    pub async fn find_active_for_buildings_rls<'e, E>(
        &self,
        executor: E,
        building_ids: &[Uuid],
    ) -> Result<Vec<OutageSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let building_ids_json = serde_json::to_value(building_ids).unwrap_or_default();

        let outages = sqlx::query_as::<_, OutageSummary>(
            r#"
            SELECT
                id, title,
                commodity::text as commodity, severity::text as severity, status::text as status,
                scheduled_start, scheduled_end, actual_start, actual_end
            FROM outages
            WHERE status IN ('planned', 'ongoing')
              AND (building_ids = '[]'::jsonb OR building_ids ?| $1)
            ORDER BY scheduled_start ASC
            "#,
        )
        .bind(&building_ids_json)
        .fetch_all(executor)
        .await?;

        Ok(outages)
    }
}

//! Fault repository (Epic 4: Fault Reporting & Resolution).
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
//! async fn create_fault(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateFaultRequest>,
//! ) -> Result<Json<Fault>> {
//!     let fault = state.fault_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(fault))
//! }
//! ```

use crate::models::fault::{
    timeline_action, AddFaultComment, AddWorkNote, AssignFault, CategoryCount, ConfirmFault,
    CreateFault, CreateFaultAttachment, CreateFaultTimelineEntry, Fault, FaultAttachment,
    FaultListQuery, FaultStatistics, FaultSummary, FaultTimelineEntry, FaultTimelineEntryWithUser,
    FaultWithDetails, PriorityCount, ReopenFault, ResolveFault, StatusCount, TriageFault,
    UpdateFault, UpdateFaultStatus,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{Error as SqlxError, Executor, FromRow, Postgres};
use uuid::Uuid;

/// Canonical fault-by-status KPI definition — the single source of truth.
///
/// Both the owner/portfolio fault statistics ([`FaultStatistics::by_status`]) and
/// the platform-admin support-data diagnostics (`SupportData.fault_by_status`)
/// derive their per-status fault counts from this one query, so the two
/// dashboards can never disagree. This unifies the dual `FaultStatusCount`
/// definition flagged in the 2026-05-28 pm-data decision — do not reintroduce a
/// second inline `GROUP BY status` count; call this instead.
///
/// Scope:
/// - `org_id = None` counts faults across **all** organisations (platform scope,
///   used by the support-data admin page).
/// - `org_id = Some(_)` scopes to a single organisation, optionally narrowed to
///   one building via `building_id`.
///
/// Rows are ordered by descending count so callers can render "top statuses"
/// directly.
pub async fn fault_counts_by_status<'e, E>(
    executor: E,
    org_id: Option<Uuid>,
    building_id: Option<Uuid>,
) -> Result<Vec<StatusCount>, SqlxError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, StatusCount>(
        r#"
        SELECT status::text AS status, COUNT(*) AS count
        FROM faults
        WHERE ($1::uuid IS NULL OR organization_id = $1)
          AND ($2::uuid IS NULL OR building_id = $2)
        GROUP BY status
        ORDER BY count DESC
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_all(executor)
    .await
}

/// Row struct for fault with details query.
#[derive(Debug, FromRow)]
struct FaultDetailsRow {
    // Fault fields
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub reporter_id: Uuid,
    pub title: String,
    pub description: String,
    pub location_description: Option<String>,
    pub category: String,
    pub priority: String,
    pub status: String,
    pub ai_category: Option<String>,
    pub ai_priority: Option<String>,
    pub ai_confidence: Option<rust_decimal::Decimal>,
    pub ai_processed_at: Option<DateTime<Utc>>,
    pub assigned_to: Option<Uuid>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub triaged_by: Option<Uuid>,
    pub triaged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub resolution_notes: Option<String>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub confirmed_by: Option<Uuid>,
    pub rating: Option<i32>,
    pub feedback: Option<String>,
    pub scheduled_date: Option<chrono::NaiveDate>,
    pub estimated_completion: Option<chrono::NaiveDate>,
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined fields
    pub reporter_name: String,
    pub reporter_email: String,
    pub building_name: String,
    pub building_address: String,
    pub unit_designation: Option<String>,
    pub assigned_to_name: Option<String>,
    pub attachment_count: i64,
    pub comment_count: i64,
}

/// Row struct for timeline entry with user.
#[derive(Debug, FromRow)]
struct TimelineEntryRow {
    pub id: Uuid,
    pub fault_id: Uuid,
    pub user_id: Uuid,
    pub action: String,
    pub note: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub metadata: JsonValue,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
}

/// Repository for fault operations.
#[derive(Clone)]
pub struct FaultRepository {
    pool: DbPool,
}

impl FaultRepository {
    /// Create a new FaultRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    /// Create a new fault with RLS context (Story 4.1).
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_rls<'e, E>(
        &self,
        executor: E,
        data: CreateFault,
    ) -> Result<Fault, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let priority = data.priority.as_deref().unwrap_or("medium");

        let fault = sqlx::query_as::<_, Fault>(
            r#"
            INSERT INTO faults (
                organization_id, building_id, unit_id, reporter_id,
                title, description, location_description,
                category, priority, idempotency_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(data.reporter_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.location_description)
        .bind(&data.category)
        .bind(priority)
        .bind(&data.idempotency_key)
        .fetch_one(executor)
        .await?;

        // Note: Timeline entry creation uses the pool since we consumed the executor.
        // For full RLS support, caller should create timeline entry separately.
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: fault.id,
            user_id: data.reporter_id,
            action: timeline_action::CREATED.to_string(),
            note: None,
            old_value: None,
            new_value: None,
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    /// Find fault by ID with RLS context.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Fault>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // The enum columns (category/priority/status/ai_*) are cast to text so the
        // row decodes into the `String`-typed `Fault` model regardless of whether
        // the connection has the PG enum types registered (matches the `::text`
        // idiom used by `find_by_id_for_org` and the statistics queries).
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            SELECT
                id, organization_id, building_id, unit_id, reporter_id,
                title, description, location_description,
                category::text AS category,
                priority::text AS priority,
                status::text AS status,
                ai_category::text AS ai_category,
                ai_priority::text AS ai_priority,
                ai_confidence, ai_processed_at,
                assigned_to, assigned_at, triaged_by, triaged_at,
                resolved_at, resolved_by, resolution_notes,
                confirmed_at, confirmed_by, rating, feedback,
                scheduled_date, estimated_completion, idempotency_key,
                created_at, updated_at
            FROM faults
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(fault)
    }

    /// Find a fault by idempotency key with RLS context (Story 4.1, #970).
    ///
    /// Use this with an `RlsConnection` so the lookup is scoped to the caller's
    /// tenant by the `faults_tenant_isolation` RLS policy. Unlike the
    /// pool-based [`find_by_idempotency_key`](Self::find_by_idempotency_key),
    /// this prevents a key collision across organizations from leaking another
    /// tenant's fault during offline-create idempotency replay.
    pub async fn find_by_idempotency_key_rls<'e, E>(
        &self,
        executor: E,
        key: &str,
    ) -> Result<Option<Fault>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Cast enum columns to text so the row decodes into the `String`-typed
        // `Fault` model (matches `find_by_id_for_org` and the statistics queries).
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            SELECT
                id, organization_id, building_id, unit_id, reporter_id,
                title, description, location_description,
                category::text AS category,
                priority::text AS priority,
                status::text AS status,
                ai_category::text AS ai_category,
                ai_priority::text AS ai_priority,
                ai_confidence, ai_processed_at,
                assigned_to, assigned_at, triaged_by, triaged_at,
                resolved_at, resolved_by, resolution_notes,
                confirmed_at, confirmed_by, rating, feedback,
                scheduled_date, estimated_completion, idempotency_key,
                created_at, updated_at
            FROM faults
            WHERE idempotency_key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(executor)
        .await?;

        Ok(fault)
    }

    /// Find a fault by ID, scoped to a specific organization.
    ///
    /// SECURITY (#770): explicitly threads `organization_id` into the WHERE
    /// clause so a caller in org B cannot read/act on org A's fault by
    /// enumerating its UUID. Returns `None` (→ 404) for a cross-tenant id.
    /// Use this for the legacy pool-based mutate-by-id handlers
    /// (assign/resolve/confirm/reopen, comments, attachments) that do NOT
    /// run under an `RlsConnection` and therefore are not protected by the
    /// `faults_tenant_isolation` RLS policy.
    pub async fn find_by_id_for_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Fault>, SqlxError> {
        // The enum columns (category/priority/status) are cast to text so the
        // row decodes into the `String`-typed `Fault` model regardless of
        // whether the connection has the PG enum types registered (matches the
        // `::text` idiom used by the statistics queries below).
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            SELECT
                id, organization_id, building_id, unit_id, reporter_id,
                title, description, location_description,
                category::text AS category,
                priority::text AS priority,
                status::text AS status,
                ai_category::text AS ai_category,
                ai_priority::text AS ai_priority,
                ai_confidence, ai_processed_at,
                assigned_to, assigned_at, triaged_by, triaged_at,
                resolved_at, resolved_by, resolution_notes,
                confirmed_at, confirmed_by, rating, feedback,
                scheduled_date, estimated_completion, idempotency_key,
                created_at, updated_at
            FROM faults
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(fault)
    }

    /// Update fault details with RLS context (reporter can edit before triage).
    pub async fn update_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateFault,
    ) -> Result<Fault, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                location_description = COALESCE($4, location_description),
                category = COALESCE($5, category),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.location_description)
        .bind(&data.category)
        .fetch_one(executor)
        .await?;

        Ok(fault)
    }

    /// Update fault status with RLS context (Story 4.4).
    pub async fn update_status_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
        data: UpdateFaultStatus,
        current_status: String,
    ) -> Result<Fault, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                status = $2,
                scheduled_date = COALESCE($3, scheduled_date),
                estimated_completion = COALESCE($4, estimated_completion),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.status)
        .bind(data.scheduled_date)
        .bind(data.estimated_completion)
        .fetch_one(executor)
        .await?;

        // Note: Timeline entry creation uses the pool since we consumed the executor.
        // For full RLS support, caller should create timeline entry separately.
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: id,
            user_id,
            action: timeline_action::STATUS_CHANGED.to_string(),
            note: data.note,
            old_value: Some(current_status),
            new_value: Some(data.status),
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    /// List faults by building with RLS context.
    pub async fn list_by_building_rls<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FaultSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let faults = sqlx::query_as::<_, FaultSummary>(
            r#"
            SELECT id, building_id, unit_id, title, category, priority, status, created_at
            FROM faults
            WHERE building_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(building_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        Ok(faults)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    /// Create a new fault (Story 4.1).
    ///
    /// **Deprecated**: Use `create_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.274", note = "Use create_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn create(&self, data: CreateFault) -> Result<Fault, SqlxError> {
        self.create_rls(&self.pool, data).await
    }

    /// Find fault by ID.
    ///
    /// **Deprecated**: Use `find_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use find_by_id_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Fault>, SqlxError> {
        self.find_by_id_rls(&self.pool, id).await
    }

    /// Find fault by idempotency key.
    pub async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Fault>, SqlxError> {
        // Cast enum columns to text so the row decodes into the `String`-typed
        // `Fault` model (matches `find_by_id_for_org` and the statistics queries).
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            SELECT
                id, organization_id, building_id, unit_id, reporter_id,
                title, description, location_description,
                category::text AS category,
                priority::text AS priority,
                status::text AS status,
                ai_category::text AS ai_category,
                ai_priority::text AS ai_priority,
                ai_confidence, ai_processed_at,
                assigned_to, assigned_at, triaged_by, triaged_at,
                resolved_at, resolved_by, resolution_notes,
                confirmed_at, confirmed_by, rating, feedback,
                scheduled_date, estimated_completion, idempotency_key,
                created_at, updated_at
            FROM faults
            WHERE idempotency_key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(fault)
    }

    /// Find fault with full details.
    pub async fn find_by_id_with_details(
        &self,
        id: Uuid,
    ) -> Result<Option<FaultWithDetails>, SqlxError> {
        let result = sqlx::query_as::<_, FaultDetailsRow>(
            r#"
            SELECT
                f.id, f.organization_id, f.building_id, f.unit_id, f.reporter_id,
                f.title, f.description, f.location_description,
                f.category::text AS category, f.priority::text AS priority, f.status::text AS status,
                f.ai_category::text AS ai_category, f.ai_priority::text AS ai_priority,
                f.ai_confidence, f.ai_processed_at,
                f.assigned_to, f.assigned_at, f.triaged_by, f.triaged_at,
                f.resolved_at, f.resolved_by, f.resolution_notes,
                f.confirmed_at, f.confirmed_by, f.rating, f.feedback,
                f.scheduled_date, f.estimated_completion, f.idempotency_key,
                f.created_at, f.updated_at,
                u.name as reporter_name,
                u.email as reporter_email,
                COALESCE(b.name, b.street) as building_name,
                b.street || ', ' || b.city as building_address,
                un.designation as unit_designation,
                au.name as assigned_to_name,
                (SELECT COUNT(*) FROM fault_attachments WHERE fault_id = f.id) as attachment_count,
                (SELECT COUNT(*) FROM fault_timeline WHERE fault_id = f.id AND action = 'comment') as comment_count
            FROM faults f
            JOIN users u ON f.reporter_id = u.id
            JOIN buildings b ON f.building_id = b.id
            LEFT JOIN units un ON f.unit_id = un.id
            LEFT JOIN users au ON f.assigned_to = au.id
            WHERE f.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(Self::details_row_to_model))
    }

    /// Find fault with full details, scoped to a specific organization.
    ///
    /// SECURITY (#770): the `get_fault` handler runs WITHOUT an
    /// `RlsConnection`, so the unscoped `find_by_id_with_details` leaked any
    /// org's fault (and its reporter PII / timeline / attachment counts) to a
    /// caller who guessed the UUID — a cross-tenant IDOR. This variant adds an
    /// explicit `f.organization_id = $2` predicate so a cross-tenant id
    /// resolves to `None` (→ 404).
    pub async fn find_by_id_with_details_for_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<FaultWithDetails>, SqlxError> {
        let result = sqlx::query_as::<_, FaultDetailsRow>(
            r#"
            SELECT
                f.id, f.organization_id, f.building_id, f.unit_id, f.reporter_id,
                f.title, f.description, f.location_description,
                f.category::text AS category, f.priority::text AS priority, f.status::text AS status,
                f.ai_category::text AS ai_category, f.ai_priority::text AS ai_priority,
                f.ai_confidence, f.ai_processed_at,
                f.assigned_to, f.assigned_at, f.triaged_by, f.triaged_at,
                f.resolved_at, f.resolved_by, f.resolution_notes,
                f.confirmed_at, f.confirmed_by, f.rating, f.feedback,
                f.scheduled_date, f.estimated_completion, f.idempotency_key,
                f.created_at, f.updated_at,
                u.name as reporter_name,
                u.email as reporter_email,
                COALESCE(b.name, b.street) as building_name,
                b.street || ', ' || b.city as building_address,
                un.designation as unit_designation,
                au.name as assigned_to_name,
                (SELECT COUNT(*) FROM fault_attachments WHERE fault_id = f.id) as attachment_count,
                (SELECT COUNT(*) FROM fault_timeline WHERE fault_id = f.id AND action = 'comment') as comment_count
            FROM faults f
            JOIN users u ON f.reporter_id = u.id
            JOIN buildings b ON f.building_id = b.id
            LEFT JOIN units un ON f.unit_id = un.id
            LEFT JOIN users au ON f.assigned_to = au.id
            WHERE f.id = $1 AND f.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(Self::details_row_to_model))
    }

    /// Map a raw [`FaultDetailsRow`] into the public [`FaultWithDetails`] model.
    fn details_row_to_model(row: FaultDetailsRow) -> FaultWithDetails {
        let fault = Fault {
            id: row.id,
            organization_id: row.organization_id,
            building_id: row.building_id,
            unit_id: row.unit_id,
            reporter_id: row.reporter_id,
            title: row.title,
            description: row.description,
            location_description: row.location_description,
            category: row.category,
            priority: row.priority,
            status: row.status,
            ai_category: row.ai_category,
            ai_priority: row.ai_priority,
            ai_confidence: row.ai_confidence,
            ai_processed_at: row.ai_processed_at,
            assigned_to: row.assigned_to,
            assigned_at: row.assigned_at,
            triaged_by: row.triaged_by,
            triaged_at: row.triaged_at,
            resolved_at: row.resolved_at,
            resolved_by: row.resolved_by,
            resolution_notes: row.resolution_notes,
            confirmed_at: row.confirmed_at,
            confirmed_by: row.confirmed_by,
            rating: row.rating,
            feedback: row.feedback,
            scheduled_date: row.scheduled_date,
            estimated_completion: row.estimated_completion,
            idempotency_key: row.idempotency_key,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        FaultWithDetails {
            fault,
            reporter_name: row.reporter_name,
            reporter_email: row.reporter_email,
            building_name: row.building_name,
            building_address: row.building_address,
            unit_designation: row.unit_designation,
            assigned_to_name: row.assigned_to_name,
            attachment_count: row.attachment_count,
            comment_count: row.comment_count,
        }
    }

    /// List faults with filters (Story 4.3).
    pub async fn list(
        &self,
        org_id: Uuid,
        query: FaultListQuery,
    ) -> Result<Vec<FaultSummary>, SqlxError> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);
        let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
        let sort_order = query.sort_order.as_deref().unwrap_or("DESC");

        // Build dynamic WHERE clause
        let mut conditions = vec!["organization_id = $1".to_string()];
        let mut param_idx = 2;

        if query.building_id.is_some() {
            conditions.push(format!("building_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.unit_id.is_some() {
            conditions.push(format!("unit_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.assigned_to.is_some() {
            conditions.push(format!("assigned_to = ${}", param_idx));
            param_idx += 1;
        }
        if query.reporter_id.is_some() {
            conditions.push(format!("reporter_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.status.is_some() {
            conditions.push(format!("status = ANY(${})::fault_status[]", param_idx));
            param_idx += 1;
        }
        if query.priority.is_some() {
            conditions.push(format!("priority = ANY(${})::fault_priority[]", param_idx));
            param_idx += 1;
        }
        if query.category.is_some() {
            conditions.push(format!("category = ANY(${})::fault_category[]", param_idx));
            param_idx += 1;
        }
        if query.search.is_some() {
            conditions.push(format!(
                "to_tsvector('simple', title || ' ' || description) @@ plainto_tsquery('simple', ${})",
                param_idx
            ));
            param_idx += 1;
        }
        if query.from_date.is_some() {
            conditions.push(format!("created_at >= ${}", param_idx));
            param_idx += 1;
        }
        if query.to_date.is_some() {
            conditions.push(format!("created_at <= ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");
        let order_clause = format!("{} {}", sort_by, sort_order);

        let sql = format!(
            r#"
            SELECT id, building_id, unit_id, title, category, priority, status, created_at
            FROM faults
            WHERE {}
            ORDER BY {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        // Build query dynamically
        let mut query_builder =
            sqlx::query_as::<_, FaultSummary>(sqlx::AssertSqlSafe(sql)).bind(org_id);

        if let Some(building_id) = query.building_id {
            query_builder = query_builder.bind(building_id);
        }
        if let Some(unit_id) = query.unit_id {
            query_builder = query_builder.bind(unit_id);
        }
        if let Some(assigned_to) = query.assigned_to {
            query_builder = query_builder.bind(assigned_to);
        }
        if let Some(reporter_id) = query.reporter_id {
            query_builder = query_builder.bind(reporter_id);
        }
        if let Some(ref status) = query.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(ref priority) = query.priority {
            query_builder = query_builder.bind(priority);
        }
        if let Some(ref category) = query.category {
            query_builder = query_builder.bind(category);
        }
        if let Some(ref search) = query.search {
            query_builder = query_builder.bind(search);
        }
        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let faults = query_builder.fetch_all(&self.pool).await?;
        Ok(faults)
    }

    /// List faults reported by a user (Story 4.5).
    pub async fn list_by_reporter(
        &self,
        reporter_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FaultSummary>, SqlxError> {
        let faults = sqlx::query_as::<_, FaultSummary>(
            r#"
            SELECT id, building_id, unit_id, title, category, priority, status, created_at
            FROM faults
            WHERE reporter_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(reporter_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(faults)
    }

    /// Update fault details (reporter can edit before triage).
    ///
    /// **Deprecated**: Use `update_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.274", note = "Use update_rls with RlsConnection instead")]
    #[allow(deprecated)]
    pub async fn update(&self, id: Uuid, data: UpdateFault) -> Result<Fault, SqlxError> {
        self.update_rls(&self.pool, id, data).await
    }

    // ========================================================================
    // Workflow Operations
    // ========================================================================

    /// Triage a fault (Story 4.3).
    pub async fn triage(
        &self,
        id: Uuid,
        triaged_by: Uuid,
        data: TriageFault,
    ) -> Result<Fault, SqlxError> {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                priority = $2,
                category = COALESCE($3, category),
                assigned_to = $4,
                assigned_at = CASE WHEN $4 IS NOT NULL THEN NOW() ELSE assigned_at END,
                triaged_by = $5,
                triaged_at = NOW(),
                status = 'triaged',
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.priority)
        .bind(&data.category)
        .bind(data.assigned_to)
        .bind(triaged_by)
        .fetch_one(&self.pool)
        .await?;

        // Create timeline entry
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: id,
            user_id: triaged_by,
            action: timeline_action::TRIAGED.to_string(),
            note: None,
            old_value: None,
            new_value: Some(format!("Priority: {}", data.priority)),
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    /// Assign fault to a user.
    pub async fn assign(
        &self,
        id: Uuid,
        assigned_by: Uuid,
        data: AssignFault,
    ) -> Result<Fault, SqlxError> {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                assigned_to = $2,
                assigned_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.assigned_to)
        .fetch_one(&self.pool)
        .await?;

        // Create timeline entry
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: id,
            user_id: assigned_by,
            action: timeline_action::ASSIGNED.to_string(),
            note: None,
            old_value: None,
            new_value: None,
            metadata: Some(serde_json::json!({ "assigned_to": data.assigned_to })),
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    /// Update fault status (Story 4.4).
    ///
    /// **Deprecated**: Use `update_status_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use update_status_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn update_status(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: UpdateFaultStatus,
    ) -> Result<Fault, SqlxError> {
        // Get current status for timeline
        let current = self
            .find_by_id_rls(&self.pool, id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;
        self.update_status_rls(&self.pool, id, user_id, data, current.status)
            .await
    }

    /// Resolve a fault (Story 4.4).
    pub async fn resolve(
        &self,
        id: Uuid,
        resolved_by: Uuid,
        data: ResolveFault,
    ) -> Result<Fault, SqlxError> {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                status = 'resolved',
                resolved_at = NOW(),
                resolved_by = $2,
                resolution_notes = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(resolved_by)
        .bind(&data.resolution_notes)
        .fetch_one(&self.pool)
        .await?;

        // Create timeline entry
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: id,
            user_id: resolved_by,
            action: timeline_action::RESOLVED.to_string(),
            note: Some(data.resolution_notes),
            old_value: None,
            new_value: None,
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    /// Confirm fault resolution (Story 4.6).
    pub async fn confirm(
        &self,
        id: Uuid,
        confirmed_by: Uuid,
        data: ConfirmFault,
    ) -> Result<Fault, SqlxError> {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                status = 'closed',
                confirmed_at = NOW(),
                confirmed_by = $2,
                rating = $3,
                feedback = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(confirmed_by)
        .bind(data.rating)
        .bind(&data.feedback)
        .fetch_one(&self.pool)
        .await?;

        // Create timeline entry
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: id,
            user_id: confirmed_by,
            action: timeline_action::CONFIRMED.to_string(),
            note: data.feedback,
            old_value: None,
            new_value: data.rating.map(|r| r.to_string()),
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    /// Reopen a fault (Story 4.6).
    pub async fn reopen(
        &self,
        id: Uuid,
        reopened_by: Uuid,
        data: ReopenFault,
    ) -> Result<Fault, SqlxError> {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                status = 'reopened',
                confirmed_at = NULL,
                confirmed_by = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        // Create timeline entry
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: id,
            user_id: reopened_by,
            action: timeline_action::REOPENED.to_string(),
            note: Some(data.reason),
            old_value: None,
            new_value: None,
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(fault)
    }

    // ========================================================================
    // AI Operations (Story 4.2)
    // ========================================================================

    /// Update AI suggestion for a fault.
    pub async fn update_ai_suggestion(
        &self,
        id: Uuid,
        category: &str,
        priority: Option<&str>,
        confidence: f64,
    ) -> Result<Fault, SqlxError> {
        let fault = sqlx::query_as::<_, Fault>(
            r#"
            UPDATE faults
            SET
                ai_category = $2,
                ai_priority = $3,
                ai_confidence = $4,
                ai_processed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(category)
        .bind(priority)
        .bind(rust_decimal::Decimal::from_f64_retain(confidence))
        .fetch_one(&self.pool)
        .await?;

        Ok(fault)
    }

    // ========================================================================
    // Attachments
    // ========================================================================

    /// Add attachment to a fault.
    pub async fn add_attachment(
        &self,
        data: CreateFaultAttachment,
    ) -> Result<FaultAttachment, SqlxError> {
        let attachment = sqlx::query_as::<_, FaultAttachment>(
            r#"
            INSERT INTO fault_attachments (
                fault_id, filename, original_filename, content_type, size_bytes,
                storage_url, thumbnail_url, uploaded_by, description, width, height
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(data.fault_id)
        .bind(&data.filename)
        .bind(&data.original_filename)
        .bind(&data.content_type)
        .bind(data.size_bytes)
        .bind(&data.storage_url)
        .bind(&data.thumbnail_url)
        .bind(data.uploaded_by)
        .bind(&data.description)
        .bind(data.width)
        .bind(data.height)
        .fetch_one(&self.pool)
        .await?;

        // Create timeline entry
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id: data.fault_id,
            user_id: data.uploaded_by,
            action: timeline_action::ATTACHMENT_ADDED.to_string(),
            note: None,
            old_value: None,
            new_value: Some(data.original_filename),
            metadata: None,
            is_internal: false,
        })
        .await?;

        Ok(attachment)
    }

    /// List attachments for a fault.
    pub async fn list_attachments(
        &self,
        fault_id: Uuid,
    ) -> Result<Vec<FaultAttachment>, SqlxError> {
        let attachments = sqlx::query_as::<_, FaultAttachment>(
            r#"
            SELECT * FROM fault_attachments
            WHERE fault_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(fault_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(attachments)
    }

    /// Delete an attachment.
    pub async fn delete_attachment(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("DELETE FROM fault_attachments WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Timeline
    // ========================================================================

    /// Create a timeline entry.
    pub async fn create_timeline_entry(
        &self,
        data: CreateFaultTimelineEntry,
    ) -> Result<FaultTimelineEntry, SqlxError> {
        let metadata = data.metadata.unwrap_or_else(|| serde_json::json!({}));

        let entry = sqlx::query_as::<_, FaultTimelineEntry>(
            r#"
            INSERT INTO fault_timeline (
                fault_id, user_id, action, note, old_value, new_value, metadata, is_internal
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.fault_id)
        .bind(data.user_id)
        .bind(&data.action)
        .bind(&data.note)
        .bind(&data.old_value)
        .bind(&data.new_value)
        .bind(&metadata)
        .bind(data.is_internal)
        .fetch_one(&self.pool)
        .await?;

        Ok(entry)
    }

    /// List timeline entries for a fault.
    pub async fn list_timeline(
        &self,
        fault_id: Uuid,
        include_internal: bool,
    ) -> Result<Vec<FaultTimelineEntryWithUser>, SqlxError> {
        let rows = if include_internal {
            sqlx::query_as::<_, TimelineEntryRow>(
                r#"
                SELECT ft.id, ft.fault_id, ft.user_id, ft.action, ft.note,
                       ft.old_value, ft.new_value, ft.metadata, ft.is_internal, ft.created_at,
                       u.name as user_name, u.email as user_email
                FROM fault_timeline ft
                JOIN users u ON ft.user_id = u.id
                WHERE ft.fault_id = $1
                ORDER BY ft.created_at
                "#,
            )
            .bind(fault_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, TimelineEntryRow>(
                r#"
                SELECT ft.id, ft.fault_id, ft.user_id, ft.action, ft.note,
                       ft.old_value, ft.new_value, ft.metadata, ft.is_internal, ft.created_at,
                       u.name as user_name, u.email as user_email
                FROM fault_timeline ft
                JOIN users u ON ft.user_id = u.id
                WHERE ft.fault_id = $1 AND ft.is_internal = false
                ORDER BY ft.created_at
                "#,
            )
            .bind(fault_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|row| {
                let entry = FaultTimelineEntry {
                    id: row.id,
                    fault_id: row.fault_id,
                    user_id: row.user_id,
                    action: row.action,
                    note: row.note,
                    old_value: row.old_value,
                    new_value: row.new_value,
                    metadata: row.metadata,
                    is_internal: row.is_internal,
                    created_at: row.created_at,
                };
                FaultTimelineEntryWithUser {
                    entry,
                    user_name: row.user_name,
                    user_email: row.user_email,
                }
            })
            .collect())
    }

    /// Add a comment to a fault.
    pub async fn add_comment(
        &self,
        fault_id: Uuid,
        user_id: Uuid,
        data: AddFaultComment,
    ) -> Result<FaultTimelineEntry, SqlxError> {
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id,
            user_id,
            action: timeline_action::COMMENT.to_string(),
            note: Some(data.note),
            old_value: None,
            new_value: None,
            metadata: None,
            is_internal: data.is_internal,
        })
        .await
    }

    /// Add a work note to a fault.
    pub async fn add_work_note(
        &self,
        fault_id: Uuid,
        user_id: Uuid,
        data: AddWorkNote,
    ) -> Result<FaultTimelineEntry, SqlxError> {
        self.create_timeline_entry(CreateFaultTimelineEntry {
            fault_id,
            user_id,
            action: timeline_action::WORK_NOTE.to_string(),
            note: Some(data.note),
            old_value: None,
            new_value: None,
            metadata: None,
            is_internal: true,
        })
        .await
    }

    // ========================================================================
    // Statistics & Analytics (Story 4.7)
    // ========================================================================

    /// Get fault statistics for an organization.
    pub async fn get_statistics(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<FaultStatistics, SqlxError> {
        // Total and open/closed counts
        let (total_count, open_count, closed_count): (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_count,
                COUNT(*) FILTER (WHERE status != 'closed') as open_count,
                COUNT(*) FILTER (WHERE status = 'closed') as closed_count
            FROM faults
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        // By status — canonical KPI definition (single source of truth).
        let by_status = fault_counts_by_status(&self.pool, Some(org_id), building_id).await?;

        // By category
        let by_category = sqlx::query_as::<_, CategoryCount>(
            r#"
            SELECT category::text as category, COUNT(*) as count
            FROM faults
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        // By priority
        let by_priority = sqlx::query_as::<_, PriorityCount>(
            r#"
            SELECT priority::text as priority, COUNT(*) as count
            FROM faults
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
            GROUP BY priority
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        // Average resolution time
        let avg_resolution: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600)
            FROM faults
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
              AND resolved_at IS NOT NULL
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        // Average rating
        let avg_rating: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(rating::float)
            FROM faults
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR building_id = $2)
              AND rating IS NOT NULL
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(FaultStatistics {
            total_count,
            open_count,
            closed_count,
            by_status,
            by_category,
            by_priority,
            average_resolution_time_hours: avg_resolution,
            average_rating: avg_rating,
        })
    }

    /// Count faults by organization.
    pub async fn count_by_organization(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM faults WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Count open faults by building.
    pub async fn count_open_by_building(&self, building_id: Uuid) -> Result<i64, SqlxError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM faults
            WHERE building_id = $1 AND status != 'closed'
            "#,
        )
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    // ========================================================================
    // Epic 55: Reporting Analytics
    // ========================================================================

    /// Get monthly fault counts for trend analysis (Epic 55, Story 55.1).
    pub async fn get_monthly_fault_counts(
        &self,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
    ) -> Result<Vec<crate::models::reports::ReportMonthlyCount>, SqlxError> {
        let rows = sqlx::query_as::<_, crate::models::reports::ReportMonthlyCount>(
            r#"
            SELECT
                EXTRACT(YEAR FROM created_at)::int4 as year,
                EXTRACT(MONTH FROM created_at)::int4 as month,
                COUNT(*)::int8 as count
            FROM faults
            WHERE organization_id = $1
              AND created_at >= $2 AND created_at <= $3
              AND ($4::uuid IS NULL OR building_id = $4)
            GROUP BY EXTRACT(YEAR FROM created_at), EXTRACT(MONTH FROM created_at)
            ORDER BY year, month
            "#,
        )
        .bind(organization_id)
        .bind(from_date)
        .bind(to_date)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get monthly average resolution times (Epic 55, Story 55.1).
    pub async fn get_monthly_resolution_times(
        &self,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
    ) -> Result<Vec<crate::models::reports::MonthlyAverage>, SqlxError> {
        let rows = sqlx::query_as::<_, crate::models::reports::MonthlyAverage>(
            r#"
            SELECT
                EXTRACT(YEAR FROM resolved_at)::int4 as year,
                EXTRACT(MONTH FROM resolved_at)::int4 as month,
                COALESCE(AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600), 0)::float8 as average
            FROM faults
            WHERE organization_id = $1
              AND resolved_at IS NOT NULL
              AND resolved_at >= $2 AND resolved_at <= $3
              AND ($4::uuid IS NULL OR building_id = $4)
            GROUP BY EXTRACT(YEAR FROM resolved_at), EXTRACT(MONTH FROM resolved_at)
            ORDER BY year, month
            "#,
        )
        .bind(organization_id)
        .bind(from_date)
        .bind(to_date)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

//! Announcement repository (Epic 6: Announcements & Communication).
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
//! async fn create_announcement(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateAnnouncementRequest>,
//! ) -> Result<Json<Announcement>> {
//!     let announcement = state.announcement_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(announcement))
//! }
//! ```

use crate::models::announcement::{
    announcement_status, AcknowledgmentStats, Announcement, AnnouncementAttachment,
    AnnouncementComment, AnnouncementListQuery, AnnouncementRead, AnnouncementStatistics,
    AnnouncementSummary, AnnouncementWithDetails, CommentWithAuthor, CommentWithAuthorRow,
    CreateAnnouncement, CreateAnnouncementAttachment, CreateComment, DeleteComment,
    UpdateAnnouncement, UserAcknowledgmentStatus,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::{Connection, Error as SqlxError, Executor, FromRow, Postgres};
use uuid::Uuid;

/// Issue #920 / #999: the single source of truth for "is the RLS-context user
/// targeted by this announcement?".
///
/// `target_type = 'all'` is org-wide; otherwise the published announcement is
/// visible only when the user (`app.current_user_id`) matches its building /
/// unit residencies or org role. This predicate is shared **verbatim** between
/// [`AnnouncementRepository::list_published_rls`] and
/// [`AnnouncementRepository::count_published_rls`] so the feed and its count can
/// never silently diverge (the previous two hand-copied predicates were a known
/// drift hazard).
///
/// It references `$1` for the organization id (the `roles` arm), so callers must
/// bind the org id as the first parameter. Expanded via `concat!` at compile
/// time, so the resulting query string stays a `'static str`.
///
/// Membership is tested as JSONB array containment —
/// `announcements.target_ids @> to_jsonb(<value>::text)` — rather than
/// `<value> IN (SELECT jsonb_array_elements_text(target_ids))`. Both express
/// "is `<value>` an element of the `target_ids` array" (Postgres treats
/// `'["a","b"]'::jsonb @> '"a"'::jsonb` as true), but only the `@>` form is a
/// sargable operator the GIN index `idx_announcements_target_ids_gin`
/// (migration 00176) can serve, so the planner can probe it instead of
/// unnesting every candidate row's array (issue #999 / #1070).
macro_rules! announcement_targeting_predicate {
    () => {
        r#"
                target_type = 'all'
                OR (target_type = 'building' AND EXISTS (
                    SELECT 1 FROM unit_residents ur
                    JOIN units u ON u.id = ur.unit_id
                    WHERE ur.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND ur.end_date IS NULL
                      AND announcements.target_ids @> to_jsonb(u.building_id::text)
                ))
                OR (target_type = 'units' AND EXISTS (
                    SELECT 1 FROM unit_residents ur
                    WHERE ur.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND ur.end_date IS NULL
                      AND announcements.target_ids @> to_jsonb(ur.unit_id::text)
                ))
                OR (target_type = 'roles' AND EXISTS (
                    SELECT 1 FROM user_memberships m
                    WHERE m.user_id = NULLIF(current_setting('app.current_user_id', true), '')::uuid
                      AND m.organization_id = $1
                      AND m.revoked_at IS NULL
                      AND (m.expires_at IS NULL OR m.expires_at > NOW())
                      AND announcements.target_ids @> to_jsonb(m.role::text)
                ))
"#
    };
}

/// Row struct for announcement with details query.
#[derive(Debug, FromRow)]
struct AnnouncementDetailsRow {
    // Announcement fields
    pub id: Uuid,
    pub organization_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content: String,
    pub target_type: String,
    pub target_ids: serde_json::Value,
    pub status: String,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub pinned_at: Option<DateTime<Utc>>,
    pub pinned_by: Option<Uuid>,
    pub comments_enabled: bool,
    pub acknowledgment_required: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined fields
    pub author_name: String,
    pub read_count: i64,
    pub acknowledged_count: i64,
    pub comment_count: i64,
    pub attachment_count: i64,
}

/// Repository for announcement operations.
#[derive(Clone)]
pub struct AnnouncementRepository {
    pool: DbPool,
}

impl AnnouncementRepository {
    /// Create a new AnnouncementRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    // ------------------------------------------------------------------------
    // Announcement CRUD
    // ------------------------------------------------------------------------

    /// Create a new announcement with RLS context (Story 6.1).
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_rls<'e, E>(
        &self,
        executor: E,
        data: CreateAnnouncement,
    ) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let target_ids_json = serde_json::to_value(&data.target_ids).unwrap_or_default();
        let comments_enabled = data.comments_enabled.unwrap_or(true);
        let acknowledgment_required = data.acknowledgment_required.unwrap_or(false);

        // Determine initial status based on scheduled_at
        let status = if data.scheduled_at.is_some() {
            announcement_status::SCHEDULED
        } else {
            announcement_status::DRAFT
        };

        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            INSERT INTO announcements (
                organization_id, author_id, title, content,
                target_type, target_ids, status, scheduled_at,
                comments_enabled, acknowledgment_required
            )
            VALUES ($1, $2, $3, $4, $5::announcement_target_type, $6, $7::announcement_status, $8, $9, $10)
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(data.organization_id)
        .bind(data.author_id)
        .bind(&data.title)
        .bind(&data.content)
        .bind(&data.target_type)
        .bind(&target_ids_json)
        .bind(status)
        .bind(data.scheduled_at)
        .bind(comments_enabled)
        .bind(acknowledgment_required)
        .fetch_one(executor)
        .await?;

        Ok(announcement)
    }

    /// Find announcement by ID with RLS context.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Announcement>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            SELECT
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            FROM announcements WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(announcement)
    }

    /// Find announcement with full details with RLS context.
    pub async fn find_by_id_with_details_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AnnouncementWithDetails>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query_as::<_, AnnouncementDetailsRow>(
            r#"
            SELECT
                a.id, a.organization_id, a.author_id, a.title, a.content,
                a.target_type::text as target_type, a.target_ids,
                a.status::text as status, a.scheduled_at, a.published_at,
                a.pinned, a.pinned_at, a.pinned_by, a.comments_enabled,
                a.acknowledgment_required, a.created_at, a.updated_at,
                u.name as author_name,
                (SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = a.id) as read_count,
                (SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = a.id AND acknowledged_at IS NOT NULL) as acknowledged_count,
                0::bigint as comment_count,
                (SELECT COUNT(*) FROM announcement_attachments WHERE announcement_id = a.id) as attachment_count
            FROM announcements a
            JOIN users u ON a.author_id = u.id
            WHERE a.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(result.map(|row| {
            let announcement = Announcement {
                id: row.id,
                organization_id: row.organization_id,
                author_id: row.author_id,
                title: row.title,
                content: row.content,
                target_type: row.target_type,
                target_ids: row.target_ids,
                status: row.status,
                scheduled_at: row.scheduled_at,
                published_at: row.published_at,
                pinned: row.pinned,
                pinned_at: row.pinned_at,
                pinned_by: row.pinned_by,
                comments_enabled: row.comments_enabled,
                acknowledgment_required: row.acknowledgment_required,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            AnnouncementWithDetails {
                announcement,
                author_name: row.author_name,
                read_count: row.read_count,
                acknowledged_count: row.acknowledged_count,
                comment_count: row.comment_count,
                attachment_count: row.attachment_count,
            }
        }))
    }

    /// List announcements with filters with RLS context.
    pub async fn list_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: AnnouncementListQuery,
    ) -> Result<Vec<AnnouncementSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        // Build dynamic WHERE clause
        let mut conditions = vec!["organization_id = $1".to_string()];
        let mut param_idx = 2;

        if query.status.is_some() {
            conditions.push(format!(
                "status = ANY(${}::announcement_status[])",
                param_idx
            ));
            param_idx += 1;
        }
        if query.target_type.is_some() {
            conditions.push(format!("target_type = ${}", param_idx));
            param_idx += 1;
        }
        if query.author_id.is_some() {
            conditions.push(format!("author_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.pinned.is_some() {
            conditions.push(format!("pinned = ${}", param_idx));
            param_idx += 1;
        }
        if query.from_date.is_some() {
            conditions.push(format!("published_at >= ${}", param_idx));
            param_idx += 1;
        }
        if query.to_date.is_some() {
            conditions.push(format!("published_at <= ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        let sql = format!(
            r#"
            SELECT
                id, title, status::text as status, target_type::text as target_type,
                published_at, pinned, comments_enabled, acknowledgment_required
            FROM announcements
            WHERE {}
            -- NULLS LAST + a created_at tiebreaker keeps the historic
            -- semantics of COALESCE(published_at, created_at) but lets the
            -- (organization_id, pinned DESC, published_at DESC) partial
            -- index actually serve the sort (issue #518).
            ORDER BY pinned DESC, published_at DESC NULLS LAST, created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );

        let mut query_builder =
            sqlx::query_as::<_, AnnouncementSummary>(sqlx::AssertSqlSafe(sql)).bind(org_id);

        if let Some(ref status) = query.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(ref target_type) = query.target_type {
            query_builder = query_builder.bind(target_type);
        }
        if let Some(author_id) = query.author_id {
            query_builder = query_builder.bind(author_id);
        }
        if let Some(pinned) = query.pinned {
            query_builder = query_builder.bind(pinned);
        }
        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let announcements = query_builder.fetch_all(executor).await?;
        Ok(announcements)
    }

    /// List published announcements for users with RLS context.
    pub async fn list_published_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<AnnouncementSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        // Issue #920: published announcements are visible to a user only when
        // the announcement's targeting includes them. The targeting predicate
        // is shared with `count_published_rls` via
        // `announcement_targeting_predicate!` so the two can never diverge.
        let announcements = sqlx::query_as::<_, AnnouncementSummary>(concat!(
            r#"
            SELECT
                id, title, status::text as status, target_type::text as target_type,
                published_at, pinned, comments_enabled, acknowledgment_required
            FROM announcements
            WHERE organization_id = $1 AND status = 'published'
              AND ("#,
            announcement_targeting_predicate!(),
            r#")
            ORDER BY pinned DESC, published_at DESC
            LIMIT $2 OFFSET $3
            "#,
        ))
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        Ok(announcements)
    }

    /// Count announcements matching filters with RLS context.
    pub async fn count_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: AnnouncementListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Build dynamic WHERE clause
        let mut conditions = vec!["organization_id = $1".to_string()];
        let mut param_idx = 2;

        if query.status.is_some() {
            conditions.push(format!(
                "status = ANY(${}::announcement_status[])",
                param_idx
            ));
            param_idx += 1;
        }
        if query.target_type.is_some() {
            conditions.push(format!("target_type = ${}", param_idx));
            param_idx += 1;
        }
        if query.author_id.is_some() {
            conditions.push(format!("author_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.pinned.is_some() {
            conditions.push(format!("pinned = ${}", param_idx));
            param_idx += 1;
        }
        if query.from_date.is_some() {
            conditions.push(format!("published_at >= ${}", param_idx));
            param_idx += 1;
        }
        if query.to_date.is_some() {
            conditions.push(format!("published_at <= ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT COUNT(*) as count FROM announcements WHERE {}",
            where_clause
        );

        let mut query_builder = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql)).bind(org_id);

        if let Some(ref status) = query.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(ref target_type) = query.target_type {
            query_builder = query_builder.bind(target_type);
        }
        if let Some(author_id) = query.author_id {
            query_builder = query_builder.bind(author_id);
        }
        if let Some(pinned) = query.pinned {
            query_builder = query_builder.bind(pinned);
        }
        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let count = query_builder.fetch_one(executor).await?;
        Ok(count)
    }

    /// Count published announcements with RLS context.
    pub async fn count_published_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Issue #920: count only announcements the RLS-context user is targeted
        // by, using the same `announcement_targeting_predicate!` as
        // `list_published_rls` so list and count agree.
        let count = sqlx::query_scalar::<_, i64>(concat!(
            r#"
            SELECT COUNT(*)
            FROM announcements
            WHERE organization_id = $1 AND status = 'published'
              AND ("#,
            announcement_targeting_predicate!(),
            r#")
            "#,
        ))
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    /// Update announcement details with RLS context.
    pub async fn update_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateAnnouncement,
    ) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let target_ids_json = data
            .target_ids
            .as_ref()
            .map(|ids| serde_json::to_value(ids).unwrap_or_default());

        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET
                title = COALESCE($2, title),
                content = COALESCE($3, content),
                target_type = COALESCE($4::announcement_target_type, target_type),
                target_ids = COALESCE($5, target_ids),
                scheduled_at = COALESCE($6, scheduled_at),
                comments_enabled = COALESCE($7, comments_enabled),
                acknowledgment_required = COALESCE($8, acknowledgment_required),
                updated_at = NOW()
            WHERE id = $1 AND status IN ('draft', 'scheduled')
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.content)
        .bind(&data.target_type)
        .bind(&target_ids_json)
        .bind(data.scheduled_at)
        .bind(data.comments_enabled)
        .bind(data.acknowledgment_required)
        .fetch_one(executor)
        .await?;

        Ok(announcement)
    }

    /// Archive an announcement with RLS context.
    pub async fn archive_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET status = 'archived', updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await?;

        Ok(announcement)
    }

    /// Delete an announcement with RLS context.
    pub async fn delete_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM announcements WHERE id = $1 AND status = 'draft'")
            .bind(id)
            .execute(executor)
            .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Publishing Operations (Story 6.1)
    // ------------------------------------------------------------------------

    /// Publish an announcement immediately with RLS context.
    pub async fn publish_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET
                status = 'published',
                published_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND status IN ('draft', 'scheduled')
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await?;

        Ok(announcement)
    }

    /// Schedule an announcement for future publishing with RLS context.
    pub async fn schedule_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        scheduled_at: DateTime<Utc>,
    ) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET
                status = 'scheduled',
                scheduled_at = $2,
                updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(scheduled_at)
        .fetch_one(executor)
        .await?;

        Ok(announcement)
    }

    /// Find scheduled announcements ready to be published with RLS context.
    pub async fn find_scheduled_for_publishing_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<Announcement>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcements = sqlx::query_as::<_, Announcement>(
            r#"
            SELECT
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            FROM announcements
            WHERE status = 'scheduled' AND scheduled_at <= NOW()
            "#,
        )
        .fetch_all(executor)
        .await?;

        Ok(announcements)
    }

    /// Publish all scheduled announcements that are due with RLS context.
    pub async fn publish_scheduled_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<Announcement>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcements = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET status = 'published', published_at = NOW(), updated_at = NOW()
            WHERE status = 'scheduled' AND scheduled_at <= NOW()
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .fetch_all(executor)
        .await?;

        Ok(announcements)
    }

    // ------------------------------------------------------------------------
    // Pinning Operations (Story 6.4)
    // ------------------------------------------------------------------------

    /// Maximum number of pinned announcements per organization.
    pub const MAX_PINNED_PER_ORG: i64 = 3;

    /// Count pinned announcements for an organization with RLS context.
    pub async fn count_pinned_rls<'e, E>(&self, executor: E, org_id: Uuid) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM announcements WHERE organization_id = $1 AND pinned = true",
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    /// Pin an announcement with RLS context.
    /// Returns error if max pinned limit (3) is reached.
    ///
    /// Note: This method requires two queries and cannot use a single executor.
    /// Use `pin` for the full implementation or handle the logic at the service layer.
    pub async fn pin_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        pinned_by: Uuid,
    ) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Pin the announcement (caller should verify pinned count beforehand)
        let pinned = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET pinned = true, pinned_at = NOW(), pinned_by = $2, updated_at = NOW()
            WHERE id = $1 AND status = 'published'
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(pinned_by)
        .fetch_one(executor)
        .await?;

        Ok(pinned)
    }

    /// Atomically check the max-3 cap and pin an announcement.
    ///
    /// Defends issue #518: the previous handler-level "SELECT count → pin"
    /// flow had a TOCTOU race where two concurrent PATCH requests could
    /// both observe `count=2`, both pass the guard, and both end up pinning
    /// — leaving the org with 4 pinned rows.
    ///
    /// The fix locks the org's pinned rows with `FOR UPDATE` inside the
    /// same transaction that performs the UPDATE, so any racing caller
    /// either sees the new row (and bumps the count) or blocks until the
    /// first transaction commits.
    ///
    /// Returns `Ok(Err(actual_count))` when the cap is already reached so
    /// the caller can map it to a 400 with the real count, and
    /// `Ok(Ok(announcement))` on success. SQL errors are surfaced via the
    /// outer `Result`.
    pub async fn pin_with_cap_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        org_id: Uuid,
        pinned_by: Uuid,
        max_pinned: i64,
    ) -> Result<Result<Announcement, i64>, SqlxError> {
        let mut tx = conn.begin().await?;

        // Lock all currently-pinned rows for this org; any concurrent pin
        // attempt blocks here until we commit. Postgres rejects `FOR UPDATE`
        // combined with an aggregate (`SELECT COUNT(*) … FOR UPDATE` →
        // SQLSTATE 0A000), so we lock the individual rows and count them in
        // Rust instead — same locking semantics, valid SQL.
        let locked_ids: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM announcements
            WHERE organization_id = $1 AND pinned = true
            FOR UPDATE
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *tx)
        .await?;
        let count = locked_ids.len() as i64;

        if count >= max_pinned {
            tx.rollback().await?;
            return Ok(Err(count));
        }

        let pinned = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET pinned = true, pinned_at = NOW(), pinned_by = $2, updated_at = NOW()
            WHERE id = $1 AND status = 'published'
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(pinned_by)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Ok(pinned))
    }

    /// Unpin an announcement with RLS context.
    pub async fn unpin_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<Announcement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let announcement = sqlx::query_as::<_, Announcement>(
            r#"
            UPDATE announcements
            SET pinned = false, pinned_at = NULL, pinned_by = NULL, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, organization_id, author_id, title, content,
                target_type::text as target_type, target_ids,
                status::text as status, scheduled_at, published_at,
                pinned, pinned_at, pinned_by, comments_enabled,
                acknowledgment_required, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await?;

        Ok(announcement)
    }

    // ------------------------------------------------------------------------
    // Attachments
    // ------------------------------------------------------------------------

    /// Add an attachment to an announcement with RLS context.
    pub async fn add_attachment_rls<'e, E>(
        &self,
        executor: E,
        data: CreateAnnouncementAttachment,
    ) -> Result<AnnouncementAttachment, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let attachment = sqlx::query_as::<_, AnnouncementAttachment>(
            r#"
            INSERT INTO announcement_attachments (announcement_id, file_key, file_name, file_type, file_size)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.announcement_id)
        .bind(&data.file_key)
        .bind(&data.file_name)
        .bind(&data.file_type)
        .bind(data.file_size)
        .fetch_one(executor)
        .await?;

        Ok(attachment)
    }

    /// Get attachments for an announcement with RLS context.
    pub async fn get_attachments_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
    ) -> Result<Vec<AnnouncementAttachment>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let attachments = sqlx::query_as::<_, AnnouncementAttachment>(
            r#"
            SELECT * FROM announcement_attachments
            WHERE announcement_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(announcement_id)
        .fetch_all(executor)
        .await?;

        Ok(attachments)
    }

    /// Delete an attachment with RLS context.
    pub async fn delete_attachment_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM announcement_attachments WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Read Tracking (Foundation for Story 6.2)
    // ------------------------------------------------------------------------

    /// Mark an announcement as read by a user with RLS context.
    pub async fn mark_read_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<AnnouncementRead, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let read = sqlx::query_as::<_, AnnouncementRead>(
            r#"
            INSERT INTO announcement_reads (announcement_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (announcement_id, user_id) DO UPDATE
            SET read_at = announcement_reads.read_at
            RETURNING *
            "#,
        )
        .bind(announcement_id)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(read)
    }

    /// Acknowledge an announcement with RLS context.
    pub async fn acknowledge_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<AnnouncementRead, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let read = sqlx::query_as::<_, AnnouncementRead>(
            r#"
            INSERT INTO announcement_reads (announcement_id, user_id, acknowledged_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (announcement_id, user_id) DO UPDATE
            SET acknowledged_at = COALESCE(announcement_reads.acknowledged_at, NOW())
            RETURNING *
            "#,
        )
        .bind(announcement_id)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(read)
    }

    /// Get read status for a user with RLS context.
    pub async fn get_read_status_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AnnouncementRead>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let read = sqlx::query_as::<_, AnnouncementRead>(
            r#"
            SELECT * FROM announcement_reads
            WHERE announcement_id = $1 AND user_id = $2
            "#,
        )
        .bind(announcement_id)
        .bind(user_id)
        .fetch_optional(executor)
        .await?;

        Ok(read)
    }

    // ------------------------------------------------------------------------
    // Statistics
    // ------------------------------------------------------------------------

    /// Get announcement statistics for an organization with RLS context.
    pub async fn get_statistics_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<AnnouncementStatistics, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let stats = sqlx::query_as::<_, AnnouncementStatistics>(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'published') as published,
                COUNT(*) FILTER (WHERE status = 'draft') as draft,
                COUNT(*) FILTER (WHERE status = 'scheduled') as scheduled,
                COUNT(*) FILTER (WHERE status = 'archived') as archived
            FROM announcements
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        Ok(stats)
    }

    /// Count unread announcements for a user with RLS context.
    pub async fn count_unread_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM announcements a
            WHERE a.organization_id = $1
              AND a.status = 'published'
              AND NOT EXISTS (
                  SELECT 1 FROM announcement_reads ar
                  WHERE ar.announcement_id = a.id AND ar.user_id = $2
              )
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(count.0)
    }

    // ------------------------------------------------------------------------
    // Acknowledgment Stats (Story 6.2)
    // ------------------------------------------------------------------------

    /// Get acknowledgment statistics for an announcement with RLS context.
    pub async fn get_acknowledgment_stats_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
    ) -> Result<AcknowledgmentStats, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Single query to get all stats. `users` has no `organization_id`
        // column — membership lives in `user_memberships` (migration 00128,
        // the canonical authz spine). Join through it and filter to active
        // grants. Same schema-drift fix as PR #322 (scheduler.rs).
        let stats: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = $1) as read_count,
                (SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = $1 AND acknowledged_at IS NOT NULL) as acknowledged_count,
                (SELECT COUNT(DISTINCT u.id) FROM users u
                 JOIN user_memberships m ON m.user_id = u.id
                 JOIN announcements a ON a.organization_id = m.organization_id
                 WHERE a.id = $1
                   AND m.revoked_at IS NULL
                   AND (m.expires_at IS NULL OR m.expires_at > NOW())
                   AND u.status = 'active') as total_targeted
            "#,
        )
        .bind(announcement_id)
        .fetch_one(executor)
        .await?;

        let pending_count = stats.2 - stats.0;

        Ok(AcknowledgmentStats {
            announcement_id,
            total_targeted: stats.2,
            read_count: stats.0,
            acknowledged_count: stats.1,
            pending_count: pending_count.max(0),
        })
    }

    /// Get list of users with their acknowledgment status with RLS context.
    pub async fn get_acknowledgment_list_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UserAcknowledgmentStatus>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        // `users` has no `organization_id` column — membership lives in
        // `user_memberships` (migration 00128, the canonical authz spine).
        // Join through it and filter to active grants. Same schema-drift
        // fix as PR #322 (scheduler.rs).
        let users = sqlx::query_as::<_, UserAcknowledgmentStatus>(
            r#"
            SELECT DISTINCT
                u.id as user_id,
                u.name as user_name,
                ar.read_at,
                ar.acknowledged_at
            FROM users u
            JOIN user_memberships m ON m.user_id = u.id
            JOIN announcements a ON a.organization_id = m.organization_id
            LEFT JOIN announcement_reads ar ON ar.announcement_id = a.id AND ar.user_id = u.id
            WHERE a.id = $1
              AND m.revoked_at IS NULL
              AND (m.expires_at IS NULL OR m.expires_at > NOW())
              AND u.status = 'active'
            ORDER BY
                ar.acknowledged_at DESC NULLS LAST,
                ar.read_at DESC NULLS LAST,
                u.name
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(announcement_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        Ok(users)
    }

    // ------------------------------------------------------------------------
    // Comments (Story 6.3)
    // ------------------------------------------------------------------------

    /// Create a new comment on an announcement with RLS context.
    pub async fn create_comment_rls<'e, E>(
        &self,
        executor: E,
        data: CreateComment,
    ) -> Result<AnnouncementComment, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let comment = sqlx::query_as::<_, AnnouncementComment>(
            r#"
            INSERT INTO announcement_comments (announcement_id, user_id, parent_id, content, ai_training_consent)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.announcement_id)
        .bind(data.user_id)
        .bind(data.parent_id)
        .bind(&data.content)
        .bind(data.ai_training_consent)
        .fetch_one(executor)
        .await?;

        Ok(comment)
    }

    /// Get a comment by ID with RLS context.
    pub async fn get_comment_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AnnouncementComment>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let comment = sqlx::query_as::<_, AnnouncementComment>(
            "SELECT * FROM announcement_comments WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(comment)
    }

    /// Get comments for an announcement with author info with RLS context.
    pub async fn get_comments_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CommentWithAuthorRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let comments = sqlx::query_as::<_, CommentWithAuthorRow>(
            r#"
            SELECT
                c.id, c.announcement_id, c.user_id, c.parent_id,
                c.content, u.name as author_name, c.deleted_at,
                c.created_at, c.updated_at
            FROM announcement_comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.announcement_id = $1 AND c.parent_id IS NULL
            ORDER BY c.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(announcement_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        Ok(comments)
    }

    /// Get replies to a comment with RLS context.
    pub async fn get_comment_replies_rls<'e, E>(
        &self,
        executor: E,
        parent_id: Uuid,
    ) -> Result<Vec<CommentWithAuthorRow>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let replies = sqlx::query_as::<_, CommentWithAuthorRow>(
            r#"
            SELECT
                c.id, c.announcement_id, c.user_id, c.parent_id,
                c.content, u.name as author_name, c.deleted_at,
                c.created_at, c.updated_at
            FROM announcement_comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.parent_id = $1
            ORDER BY c.created_at ASC
            "#,
        )
        .bind(parent_id)
        .fetch_all(executor)
        .await?;

        Ok(replies)
    }

    /// Soft-delete a comment with RLS context.
    pub async fn delete_comment_rls<'e, E>(
        &self,
        executor: E,
        data: DeleteComment,
    ) -> Result<AnnouncementComment, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let comment = sqlx::query_as::<_, AnnouncementComment>(
            r#"
            UPDATE announcement_comments
            SET deleted_at = NOW(), deleted_by = $2, deletion_reason = $3, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(data.comment_id)
        .bind(data.deleted_by)
        .bind(&data.deletion_reason)
        .fetch_one(executor)
        .await?;

        Ok(comment)
    }

    /// Get comment count for an announcement with RLS context.
    pub async fn get_comment_count_rls<'e, E>(
        &self,
        executor: E,
        announcement_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM announcement_comments WHERE announcement_id = $1 AND deleted_at IS NULL",
        )
        .bind(announcement_id)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    // ------------------------------------------------------------------------
    // Announcement CRUD
    // ------------------------------------------------------------------------

    /// Create a new announcement (Story 6.1).
    ///
    /// **Deprecated**: Use `create_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use create_rls with RlsConnection instead")]
    pub async fn create(&self, data: CreateAnnouncement) -> Result<Announcement, SqlxError> {
        self.create_rls(&self.pool, data).await
    }

    /// Find announcement by ID.
    ///
    /// **Deprecated**: Use `find_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Announcement>, SqlxError> {
        self.find_by_id_rls(&self.pool, id).await
    }

    /// Find announcement with full details.
    ///
    /// **Deprecated**: Use `find_by_id_with_details_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_by_id_with_details_rls with RlsConnection instead"
    )]
    pub async fn find_by_id_with_details(
        &self,
        id: Uuid,
    ) -> Result<Option<AnnouncementWithDetails>, SqlxError> {
        self.find_by_id_with_details_rls(&self.pool, id).await
    }

    /// List announcements with filters.
    ///
    /// **Deprecated**: Use `list_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use list_rls with RlsConnection instead")]
    pub async fn list(
        &self,
        org_id: Uuid,
        query: AnnouncementListQuery,
    ) -> Result<Vec<AnnouncementSummary>, SqlxError> {
        self.list_rls(&self.pool, org_id, query).await
    }

    /// List published announcements for users (respecting targeting).
    ///
    /// **Deprecated**: Use `list_published_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_published_rls with RlsConnection instead"
    )]
    pub async fn list_published(
        &self,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<AnnouncementSummary>, SqlxError> {
        self.list_published_rls(&self.pool, org_id, limit, offset)
            .await
    }

    /// Count announcements matching filters (for pagination).
    ///
    /// **Deprecated**: Use `count_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use count_rls with RlsConnection instead")]
    pub async fn count(
        &self,
        org_id: Uuid,
        query: AnnouncementListQuery,
    ) -> Result<i64, SqlxError> {
        self.count_rls(&self.pool, org_id, query).await
    }

    /// Count published announcements (for pagination).
    ///
    /// **Deprecated**: Use `count_published_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_published_rls with RlsConnection instead"
    )]
    pub async fn count_published(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        self.count_published_rls(&self.pool, org_id).await
    }

    /// Update announcement details (only in draft/scheduled status).
    ///
    /// **Deprecated**: Use `update_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use update_rls with RlsConnection instead")]
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateAnnouncement,
    ) -> Result<Announcement, SqlxError> {
        self.update_rls(&self.pool, id, data).await
    }

    /// Archive an announcement (soft delete).
    ///
    /// **Deprecated**: Use `archive_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use archive_rls with RlsConnection instead")]
    pub async fn archive(&self, id: Uuid) -> Result<Announcement, SqlxError> {
        self.archive_rls(&self.pool, id).await
    }

    /// Delete an announcement (only in draft status).
    ///
    /// **Deprecated**: Use `delete_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use delete_rls with RlsConnection instead")]
    pub async fn delete(&self, id: Uuid) -> Result<(), SqlxError> {
        self.delete_rls(&self.pool, id).await
    }

    // ------------------------------------------------------------------------
    // Publishing Operations (Story 6.1)
    // ------------------------------------------------------------------------

    /// Publish an announcement immediately.
    ///
    /// **Deprecated**: Use `publish_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use publish_rls with RlsConnection instead")]
    pub async fn publish(&self, id: Uuid) -> Result<Announcement, SqlxError> {
        self.publish_rls(&self.pool, id).await
    }

    /// Schedule an announcement for future publishing.
    ///
    /// **Deprecated**: Use `schedule_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use schedule_rls with RlsConnection instead"
    )]
    pub async fn schedule(
        &self,
        id: Uuid,
        scheduled_at: DateTime<Utc>,
    ) -> Result<Announcement, SqlxError> {
        self.schedule_rls(&self.pool, id, scheduled_at).await
    }

    /// Find scheduled announcements ready to be published.
    ///
    /// **Deprecated**: Use `find_scheduled_for_publishing_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_scheduled_for_publishing_rls with RlsConnection instead"
    )]
    pub async fn find_scheduled_for_publishing(&self) -> Result<Vec<Announcement>, SqlxError> {
        self.find_scheduled_for_publishing_rls(&self.pool).await
    }

    /// Publish all scheduled announcements that are due.
    ///
    /// **Deprecated**: Use `publish_scheduled_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use publish_scheduled_rls with RlsConnection instead"
    )]
    pub async fn publish_scheduled(&self) -> Result<Vec<Announcement>, SqlxError> {
        self.publish_scheduled_rls(&self.pool).await
    }

    // ------------------------------------------------------------------------
    // Pinning Operations (Story 6.4)
    // ------------------------------------------------------------------------

    /// Count pinned announcements for an organization.
    ///
    /// **Deprecated**: Use `count_pinned_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_pinned_rls with RlsConnection instead"
    )]
    pub async fn count_pinned(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        self.count_pinned_rls(&self.pool, org_id).await
    }

    /// Pin an announcement.
    /// Returns error if max pinned limit (3) is reached.
    #[allow(deprecated)]
    pub async fn pin(&self, id: Uuid, pinned_by: Uuid) -> Result<Announcement, SqlxError> {
        // Get announcement to check org_id and if already pinned
        let announcement = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Check if it's published
        if announcement.status != "published" {
            return Err(SqlxError::RowNotFound);
        }

        // If already pinned, just return it
        if announcement.pinned {
            return Ok(announcement);
        }

        // Check pinned count
        let pinned_count = self.count_pinned(announcement.organization_id).await?;
        if pinned_count >= Self::MAX_PINNED_PER_ORG {
            // Return a custom error - the caller should interpret this
            return Err(SqlxError::Protocol(format!(
                "Maximum of {} pinned announcements reached",
                Self::MAX_PINNED_PER_ORG
            )));
        }

        // Pin the announcement
        self.pin_rls(&self.pool, id, pinned_by).await
    }

    /// Unpin an announcement.
    ///
    /// **Deprecated**: Use `unpin_rls` with an RLS-enabled connection instead.
    #[deprecated(since = "0.2.276", note = "Use unpin_rls with RlsConnection instead")]
    pub async fn unpin(&self, id: Uuid) -> Result<Announcement, SqlxError> {
        self.unpin_rls(&self.pool, id).await
    }

    /// Auto-unpin announcements that have been pinned longer than `max_age`
    /// (Story 6.4 / issue #972.7). Returns the IDs of the announcements that
    /// were unpinned.
    ///
    /// Runs from the background scheduler without user context — like
    /// `publish_scheduled`, these are privileged/admin-level maintenance
    /// operations and use the internal pool directly (no RLS enforcement).
    pub async fn auto_unpin_expired(
        &self,
        max_age: chrono::Duration,
    ) -> Result<Vec<Uuid>, SqlxError> {
        // Build the Postgres interval from whole seconds via `make_interval`
        // (matches the codebase's `make_interval`/`|| ' days'` convention and
        // avoids relying on chrono<->interval Encode impls).
        let max_age_secs = max_age.num_seconds().max(0) as f64;

        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            UPDATE announcements
            SET pinned = false, pinned_at = NULL, pinned_by = NULL, updated_at = NOW()
            WHERE pinned = true AND pinned_at < NOW() - make_interval(secs => $1)
            RETURNING id
            "#,
        )
        .bind(max_age_secs)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ------------------------------------------------------------------------
    // Attachments
    // ------------------------------------------------------------------------

    /// Add an attachment to an announcement.
    ///
    /// **Deprecated**: Use `add_attachment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use add_attachment_rls with RlsConnection instead"
    )]
    pub async fn add_attachment(
        &self,
        data: CreateAnnouncementAttachment,
    ) -> Result<AnnouncementAttachment, SqlxError> {
        self.add_attachment_rls(&self.pool, data).await
    }

    /// Get attachments for an announcement.
    ///
    /// **Deprecated**: Use `get_attachments_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_attachments_rls with RlsConnection instead"
    )]
    pub async fn get_attachments(
        &self,
        announcement_id: Uuid,
    ) -> Result<Vec<AnnouncementAttachment>, SqlxError> {
        self.get_attachments_rls(&self.pool, announcement_id).await
    }

    /// Delete an attachment.
    ///
    /// **Deprecated**: Use `delete_attachment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use delete_attachment_rls with RlsConnection instead"
    )]
    pub async fn delete_attachment(&self, id: Uuid) -> Result<(), SqlxError> {
        self.delete_attachment_rls(&self.pool, id).await
    }

    // ------------------------------------------------------------------------
    // Read Tracking (Foundation for Story 6.2)
    // ------------------------------------------------------------------------

    /// Mark an announcement as read by a user.
    ///
    /// **Deprecated**: Use `mark_read_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use mark_read_rls with RlsConnection instead"
    )]
    pub async fn mark_read(
        &self,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<AnnouncementRead, SqlxError> {
        self.mark_read_rls(&self.pool, announcement_id, user_id)
            .await
    }

    /// Acknowledge an announcement.
    ///
    /// **Deprecated**: Use `acknowledge_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use acknowledge_rls with RlsConnection instead"
    )]
    pub async fn acknowledge(
        &self,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<AnnouncementRead, SqlxError> {
        self.acknowledge_rls(&self.pool, announcement_id, user_id)
            .await
    }

    /// Get read status for a user.
    ///
    /// **Deprecated**: Use `get_read_status_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_read_status_rls with RlsConnection instead"
    )]
    pub async fn get_read_status(
        &self,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AnnouncementRead>, SqlxError> {
        self.get_read_status_rls(&self.pool, announcement_id, user_id)
            .await
    }

    // ------------------------------------------------------------------------
    // Statistics
    // ------------------------------------------------------------------------

    /// Get announcement statistics for an organization.
    ///
    /// **Deprecated**: Use `get_statistics_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_statistics_rls with RlsConnection instead"
    )]
    pub async fn get_statistics(&self, org_id: Uuid) -> Result<AnnouncementStatistics, SqlxError> {
        self.get_statistics_rls(&self.pool, org_id).await
    }

    /// Count unread announcements for a user.
    ///
    /// **Deprecated**: Use `count_unread_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_unread_rls with RlsConnection instead"
    )]
    pub async fn count_unread(&self, org_id: Uuid, user_id: Uuid) -> Result<i64, SqlxError> {
        self.count_unread_rls(&self.pool, org_id, user_id).await
    }

    // ------------------------------------------------------------------------
    // Acknowledgment Stats (Story 6.2)
    // ------------------------------------------------------------------------

    /// Get acknowledgment statistics for an announcement (for managers).
    ///
    /// **Deprecated**: Use `get_acknowledgment_stats_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_acknowledgment_stats_rls with RlsConnection instead"
    )]
    pub async fn get_acknowledgment_stats(
        &self,
        announcement_id: Uuid,
    ) -> Result<AcknowledgmentStats, SqlxError> {
        self.get_acknowledgment_stats_rls(&self.pool, announcement_id)
            .await
    }

    /// Get list of users with their acknowledgment status for an announcement.
    ///
    /// **Deprecated**: Use `get_acknowledgment_list_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_acknowledgment_list_rls with RlsConnection instead"
    )]
    pub async fn get_acknowledgment_list(
        &self,
        announcement_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UserAcknowledgmentStatus>, SqlxError> {
        self.get_acknowledgment_list_rls(&self.pool, announcement_id, limit, offset)
            .await
    }

    // ------------------------------------------------------------------------
    // Comments (Story 6.3)
    // ------------------------------------------------------------------------

    /// Create a new comment on an announcement.
    ///
    /// **Deprecated**: Use `create_comment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_comment_rls with RlsConnection instead"
    )]
    pub async fn create_comment(
        &self,
        data: CreateComment,
    ) -> Result<AnnouncementComment, SqlxError> {
        self.create_comment_rls(&self.pool, data).await
    }

    /// Get a comment by ID.
    ///
    /// **Deprecated**: Use `get_comment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_comment_rls with RlsConnection instead"
    )]
    pub async fn get_comment(&self, id: Uuid) -> Result<Option<AnnouncementComment>, SqlxError> {
        self.get_comment_rls(&self.pool, id).await
    }

    /// Get comments for an announcement with author info.
    /// Returns top-level comments only; replies are fetched separately.
    ///
    /// **Deprecated**: Use `get_comments_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_comments_rls with RlsConnection instead"
    )]
    pub async fn get_comments(
        &self,
        announcement_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CommentWithAuthorRow>, SqlxError> {
        self.get_comments_rls(&self.pool, announcement_id, limit, offset)
            .await
    }

    /// Get replies to a comment.
    ///
    /// **Deprecated**: Use `get_comment_replies_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_comment_replies_rls with RlsConnection instead"
    )]
    pub async fn get_comment_replies(
        &self,
        parent_id: Uuid,
    ) -> Result<Vec<CommentWithAuthorRow>, SqlxError> {
        self.get_comment_replies_rls(&self.pool, parent_id).await
    }

    /// Get threaded comments (top-level with nested replies).
    ///
    /// Note: This method uses multiple queries and cannot be easily converted to RLS.
    /// Consider using `get_comments_rls` and `get_comment_replies_rls` at the service layer.
    #[allow(deprecated)]
    pub async fn get_threaded_comments(
        &self,
        announcement_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<CommentWithAuthor>, SqlxError> {
        // Get top-level comments
        let top_level = self.get_comments(announcement_id, limit, offset).await?;

        // Collect all parent IDs
        let parent_ids: Vec<Uuid> = top_level.iter().map(|c| c.id).collect();

        // Get all replies in one query
        let all_replies = if !parent_ids.is_empty() {
            sqlx::query_as::<_, CommentWithAuthorRow>(
                r#"
                SELECT
                    c.id, c.announcement_id, c.user_id, c.parent_id,
                    c.content, u.name as author_name, c.deleted_at,
                    c.created_at, c.updated_at
                FROM announcement_comments c
                JOIN users u ON c.user_id = u.id
                WHERE c.parent_id = ANY($1)
                ORDER BY c.created_at ASC
                "#,
            )
            .bind(&parent_ids)
            .fetch_all(&self.pool)
            .await?
        } else {
            vec![]
        };

        // Group replies by parent_id
        let mut replies_map: std::collections::HashMap<Uuid, Vec<CommentWithAuthor>> =
            std::collections::HashMap::new();
        for reply in all_replies {
            if let Some(parent_id) = reply.parent_id {
                replies_map
                    .entry(parent_id)
                    .or_default()
                    .push(reply.into_comment_with_author(None));
            }
        }

        // Build threaded structure
        let threaded: Vec<CommentWithAuthor> = top_level
            .into_iter()
            .map(|comment| {
                let replies = replies_map.remove(&comment.id);
                comment.into_comment_with_author(replies)
            })
            .collect();

        Ok(threaded)
    }

    /// Soft-delete a comment (author or manager moderation).
    ///
    /// **Deprecated**: Use `delete_comment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use delete_comment_rls with RlsConnection instead"
    )]
    pub async fn delete_comment(
        &self,
        data: DeleteComment,
    ) -> Result<AnnouncementComment, SqlxError> {
        self.delete_comment_rls(&self.pool, data).await
    }

    /// Get comment count for an announcement (excluding deleted).
    ///
    /// **Deprecated**: Use `get_comment_count_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_comment_count_rls with RlsConnection instead"
    )]
    pub async fn get_comment_count(&self, announcement_id: Uuid) -> Result<i64, SqlxError> {
        self.get_comment_count_rls(&self.pool, announcement_id)
            .await
    }
}

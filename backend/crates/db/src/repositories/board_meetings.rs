//! Epic 143: Board Meeting Management Repository
//!
//! This module provides database operations for board meeting management.
//!
//! # RLS Integration (PAP-137 / PAP-67 / PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the board-meeting tables (`board_members`,
//! `board_meetings`, `meeting_agenda_items`, `meeting_motions`, `motion_votes`,
//! `meeting_attendance`, `meeting_minutes`, `meeting_action_items`,
//! `meeting_documents`). Under `FORCE` the api-server's owner connection is no
//! longer exempt, so a query issued on a connection without
//! `app.current_org_id` set collapses to deny-all.
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds
//! **no pool**, so there is no way to issue a query that bypasses RLS. This
//! mirrors the `vendor.rs` / `work_order.rs` / `budget.rs` precedent.
//!
//! Cross-tenant safety (PAP-133 defense-in-depth): RLS is the primary
//! boundary, but by-id queries stay org-keyed — a connection whose role
//! bypasses RLS (superuser pools, BYPASSRLS) must still never resolve another
//! tenant's row. `board_members` and `board_meetings` carry their own
//! `organization_id`; child tables (`meeting_agenda_items`, `meeting_motions`,
//! `motion_votes`, `meeting_attendance`, `meeting_minutes`,
//! `meeting_action_items`, `meeting_documents`) are keyed through the root
//! `board_meetings.organization_id` via `EXISTS`.

use sqlx::{Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

use crate::models::board_meetings::{
    ActionItemQuery, ActionItemSummary, AttendanceHistory, BoardMeeting, BoardMember,
    BoardMemberQuery, BoardMemberSummary, CastVote, CreateActionItem, CreateAgendaItem,
    CreateBoardMeeting, CreateBoardMember, CreateMinutes, CreateMotion, MeetingActionItem,
    MeetingAgendaItem, MeetingAttendance, MeetingDashboard, MeetingDetail, MeetingDocument,
    MeetingMinutes, MeetingMotion, MeetingQuery, MeetingStatistics, MeetingSummary,
    MeetingTypeCount, MotionDetail, MotionQuery, MotionStatus, MotionStatusCount, MotionSummary,
    MotionVote, RecordAttendance, UpdateActionItem, UpdateAgendaItem, UpdateBoardMeeting,
    UpdateBoardMember, UpdateMinutes, UpdateMotion, UploadMeetingDocument, VoteSummary,
};

/// Repository for board meeting operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Debug, Clone)]
pub struct BoardMeetingRepository;

impl BoardMeetingRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ========================================================================
    // BOARD MEMBERS
    // ========================================================================

    /// Create a new board member.
    pub async fn create_board_member<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        appointed_by: Uuid,
        input: CreateBoardMember,
    ) -> Result<BoardMember, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMember>(
            r#"
            INSERT INTO board_members (
                organization_id, user_id, role, title, term_start, term_end,
                email_notifications, sms_notifications, appointed_by, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.user_id)
        .bind(input.role)
        .bind(input.title)
        .bind(input.term_start)
        .bind(input.term_end)
        .bind(input.email_notifications.unwrap_or(true))
        .bind(input.sms_notifications.unwrap_or(false))
        .bind(appointed_by)
        .bind(input.notes)
        .fetch_one(executor)
        .await
    }

    /// Get a board member by ID, scoped to the owning organization.
    pub async fn get_board_member<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<BoardMember>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMember>(
            "SELECT * FROM board_members WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List board members with optional filters.
    pub async fn list_board_members<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: BoardMemberQuery,
    ) -> Result<Vec<BoardMemberSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(50).min(100);
        let offset = (page - 1) * per_page;

        sqlx::query_as::<_, BoardMemberSummary>(
            r#"
            SELECT
                bm.id, bm.user_id, bm.role, bm.title,
                u.name as user_name, u.email as user_email,
                bm.term_start, bm.term_end, bm.is_active
            FROM board_members bm
            LEFT JOIN users u ON u.id = bm.user_id
            WHERE bm.organization_id = $1
                AND ($2::board_role IS NULL OR bm.role = $2)
                AND ($3::bool IS NULL OR bm.is_active = $3)
            ORDER BY bm.role, bm.term_start DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(query.role)
        .bind(query.is_active)
        .bind(per_page)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update a board member, scoped to the owning organization.
    ///
    /// Returns `None` when no row matches `(id, org_id)` — including the
    /// cross-tenant case, which is indistinguishable from "not found".
    pub async fn update_board_member<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        input: UpdateBoardMember,
    ) -> Result<Option<BoardMember>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMember>(
            r#"
            UPDATE board_members SET
                role = COALESCE($2, role),
                title = COALESCE($3, title),
                term_end = COALESCE($4, term_end),
                is_active = COALESCE($5, is_active),
                email_notifications = COALESCE($6, email_notifications),
                sms_notifications = COALESCE($7, sms_notifications),
                notes = COALESCE($8, notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $9
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.role)
        .bind(input.title)
        .bind(input.term_end)
        .bind(input.is_active)
        .bind(input.email_notifications)
        .bind(input.sms_notifications)
        .bind(input.notes)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Delete a board member, scoped to the owning organization.
    pub async fn delete_board_member<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM board_members WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // BOARD MEETINGS
    // ========================================================================

    /// Create a new board meeting.
    pub async fn create_meeting<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        created_by: Uuid,
        input: CreateBoardMeeting,
    ) -> Result<BoardMeeting, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMeeting>(
            r#"
            INSERT INTO board_meetings (
                organization_id, building_id, meeting_number, title, meeting_type,
                scheduled_start, scheduled_end, timezone, location_type, physical_location,
                virtual_meeting_url, virtual_meeting_id, dial_in_number, quorum_required,
                is_recorded, secretary_id, description, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.building_id)
        .bind(input.meeting_number)
        .bind(input.title)
        .bind(input.meeting_type)
        .bind(input.scheduled_start)
        .bind(input.scheduled_end)
        .bind(input.timezone)
        .bind(input.location_type)
        .bind(input.physical_location)
        .bind(input.virtual_meeting_url)
        .bind(input.virtual_meeting_id)
        .bind(input.dial_in_number)
        .bind(input.quorum_required)
        .bind(input.is_recorded.unwrap_or(false))
        .bind(input.secretary_id)
        .bind(input.description)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Get a meeting by ID, scoped to the owning organization.
    pub async fn get_meeting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<BoardMeeting>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMeeting>(
            "SELECT * FROM board_meetings WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get meeting detail with all related data.
    ///
    /// Org validation happens on the root meeting fetch: when the meeting does
    /// not belong to `org_id` this returns `Ok(None)` and no child data is
    /// read. The child queries stay org-keyed regardless (defense-in-depth).
    pub async fn get_meeting_detail(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingDetail>, sqlx::Error> {
        let meeting = match self.get_meeting(&mut *conn, id, org_id).await? {
            Some(m) => m,
            None => return Ok(None),
        };

        let agenda_items = self.list_agenda_items(&mut *conn, id, org_id).await?;
        let motions = self.list_meeting_motions(&mut *conn, id, org_id).await?;
        let attendance = self.list_meeting_attendance(&mut *conn, id, org_id).await?;
        let action_items = self
            .list_meeting_action_items(&mut *conn, id, org_id)
            .await?;
        let documents = self.list_meeting_documents(&mut *conn, id, org_id).await?;
        let minutes = self.get_meeting_minutes(&mut *conn, id, org_id).await?;

        Ok(Some(MeetingDetail {
            meeting,
            agenda_items,
            motions,
            attendance,
            action_items,
            documents,
            minutes,
        }))
    }

    /// List meetings with optional filters.
    pub async fn list_meetings<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: MeetingQuery,
    ) -> Result<Vec<MeetingSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).min(100);
        let offset = (page - 1) * per_page;

        sqlx::query_as::<_, MeetingSummary>(
            r#"
            SELECT
                m.id, m.meeting_number, m.title, m.meeting_type, m.status,
                m.scheduled_start, m.scheduled_end, m.location_type, m.quorum_met,
                (SELECT COUNT(*) FROM meeting_agenda_items WHERE meeting_id = m.id) as agenda_item_count,
                (SELECT COUNT(*) FROM meeting_motions WHERE meeting_id = m.id) as motion_count
            FROM board_meetings m
            WHERE m.organization_id = $1
                AND ($2::uuid IS NULL OR m.building_id = $2)
                AND ($3::meeting_type IS NULL OR m.meeting_type = $3)
                AND ($4::meeting_status IS NULL OR m.status = $4)
                AND ($5::timestamptz IS NULL OR m.scheduled_start >= $5)
                AND ($6::timestamptz IS NULL OR m.scheduled_start <= $6)
            ORDER BY m.scheduled_start DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.meeting_type)
        .bind(query.status)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(per_page)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update a meeting, scoped to the owning organization.
    pub async fn update_meeting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        input: UpdateBoardMeeting,
    ) -> Result<Option<BoardMeeting>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMeeting>(
            r#"
            UPDATE board_meetings SET
                meeting_number = COALESCE($2, meeting_number),
                title = COALESCE($3, title),
                meeting_type = COALESCE($4, meeting_type),
                status = COALESCE($5, status),
                scheduled_start = COALESCE($6, scheduled_start),
                scheduled_end = COALESCE($7, scheduled_end),
                actual_start = COALESCE($8, actual_start),
                actual_end = COALESCE($9, actual_end),
                timezone = COALESCE($10, timezone),
                location_type = COALESCE($11, location_type),
                physical_location = COALESCE($12, physical_location),
                virtual_meeting_url = COALESCE($13, virtual_meeting_url),
                virtual_meeting_id = COALESCE($14, virtual_meeting_id),
                dial_in_number = COALESCE($15, dial_in_number),
                quorum_required = COALESCE($16, quorum_required),
                quorum_present = COALESCE($17, quorum_present),
                quorum_met = COALESCE($18, quorum_met),
                is_recorded = COALESCE($19, is_recorded),
                recording_url = COALESCE($20, recording_url),
                secretary_id = COALESCE($21, secretary_id),
                description = COALESCE($22, description),
                notes = COALESCE($23, notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $24
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.meeting_number)
        .bind(input.title)
        .bind(input.meeting_type)
        .bind(input.status)
        .bind(input.scheduled_start)
        .bind(input.scheduled_end)
        .bind(input.actual_start)
        .bind(input.actual_end)
        .bind(input.timezone)
        .bind(input.location_type)
        .bind(input.physical_location)
        .bind(input.virtual_meeting_url)
        .bind(input.virtual_meeting_id)
        .bind(input.dial_in_number)
        .bind(input.quorum_required)
        .bind(input.quorum_present)
        .bind(input.quorum_met)
        .bind(input.is_recorded)
        .bind(input.recording_url)
        .bind(input.secretary_id)
        .bind(input.description)
        .bind(input.notes)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Start a meeting, scoped to the owning organization.
    pub async fn start_meeting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<BoardMeeting>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMeeting>(
            r#"
            UPDATE board_meetings SET
                status = 'in_progress',
                actual_start = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// End a meeting, scoped to the owning organization.
    pub async fn end_meeting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<BoardMeeting>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMeeting>(
            r#"
            UPDATE board_meetings SET
                status = 'completed',
                actual_end = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Cancel a meeting, scoped to the owning organization.
    pub async fn cancel_meeting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<BoardMeeting>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BoardMeeting>(
            r#"
            UPDATE board_meetings SET
                status = 'cancelled',
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Delete a meeting, scoped to the owning organization.
    pub async fn delete_meeting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM board_meetings WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // AGENDA ITEMS
    // ========================================================================

    /// Add an agenda item.
    ///
    /// Callers must validate that `input.meeting_id` belongs to the caller's
    /// organization first (root-meeting fetch via [`Self::get_meeting`]).
    pub async fn add_agenda_item<'e, E>(
        &self,
        executor: E,
        added_by: Uuid,
        input: CreateAgendaItem,
    ) -> Result<MeetingAgendaItem, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAgendaItem>(
            r#"
            INSERT INTO meeting_agenda_items (
                meeting_id, item_number, title, description, item_type,
                display_order, estimated_duration_minutes, presenter_id,
                presenter_name, document_ids, added_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(input.meeting_id)
        .bind(input.item_number)
        .bind(input.title)
        .bind(input.description)
        .bind(input.item_type)
        .bind(input.display_order.unwrap_or(0))
        .bind(input.estimated_duration_minutes)
        .bind(input.presenter_id)
        .bind(input.presenter_name)
        .bind(input.document_ids.as_deref())
        .bind(added_by)
        .fetch_one(executor)
        .await
    }

    /// Get an agenda item, keyed through the root meeting's organization.
    pub async fn get_agenda_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingAgendaItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAgendaItem>(
            r#"
            SELECT i.* FROM meeting_agenda_items i
            WHERE i.id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = i.meeting_id AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List agenda items for a meeting, keyed through the root meeting's
    /// organization.
    pub async fn list_agenda_items<'e, E>(
        &self,
        executor: E,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MeetingAgendaItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAgendaItem>(
            r#"
            SELECT i.* FROM meeting_agenda_items i
            WHERE i.meeting_id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = i.meeting_id AND bm.organization_id = $2
                )
            ORDER BY i.display_order
            "#,
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Update an agenda item, keyed through the root meeting's organization.
    pub async fn update_agenda_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        input: UpdateAgendaItem,
    ) -> Result<Option<MeetingAgendaItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAgendaItem>(
            r#"
            UPDATE meeting_agenda_items SET
                item_number = COALESCE($2, item_number),
                title = COALESCE($3, title),
                description = COALESCE($4, description),
                item_type = COALESCE($5, item_type),
                display_order = COALESCE($6, display_order),
                estimated_duration_minutes = COALESCE($7, estimated_duration_minutes),
                actual_duration_minutes = COALESCE($8, actual_duration_minutes),
                status = COALESCE($9, status),
                presenter_id = COALESCE($10, presenter_id),
                presenter_name = COALESCE($11, presenter_name),
                document_ids = COALESCE($12, document_ids),
                discussion_notes = COALESCE($13, discussion_notes),
                outcome = COALESCE($14, outcome),
                follow_up_required = COALESCE($15, follow_up_required),
                follow_up_assignee = COALESCE($16, follow_up_assignee),
                follow_up_due_date = COALESCE($17, follow_up_due_date),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_agenda_items.meeting_id
                        AND bm.organization_id = $18
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.item_number)
        .bind(input.title)
        .bind(input.description)
        .bind(input.item_type)
        .bind(input.display_order)
        .bind(input.estimated_duration_minutes)
        .bind(input.actual_duration_minutes)
        .bind(input.status)
        .bind(input.presenter_id)
        .bind(input.presenter_name)
        .bind(input.document_ids.as_deref())
        .bind(input.discussion_notes)
        .bind(input.outcome)
        .bind(input.follow_up_required)
        .bind(input.follow_up_assignee)
        .bind(input.follow_up_due_date)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Complete an agenda item, keyed through the root meeting's organization.
    pub async fn complete_agenda_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        outcome: Option<String>,
    ) -> Result<Option<MeetingAgendaItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAgendaItem>(
            r#"
            UPDATE meeting_agenda_items SET
                status = 'completed',
                outcome = COALESCE($2, outcome),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_agenda_items.meeting_id
                        AND bm.organization_id = $3
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(outcome)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Delete an agenda item, keyed through the root meeting's organization.
    pub async fn delete_agenda_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM meeting_agenda_items
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_agenda_items.meeting_id
                        AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // MOTIONS
    // ========================================================================

    /// Create a motion.
    ///
    /// Callers must validate that `input.meeting_id` belongs to the caller's
    /// organization first (root-meeting fetch via [`Self::get_meeting`]).
    pub async fn create_motion<'e, E>(
        &self,
        executor: E,
        proposed_by: Uuid,
        input: CreateMotion,
    ) -> Result<MeetingMotion, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMotion>(
            r#"
            INSERT INTO meeting_motions (
                meeting_id, agenda_item_id, motion_number, title, motion_text,
                proposed_by, effective_date, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(input.meeting_id)
        .bind(input.agenda_item_id)
        .bind(input.motion_number)
        .bind(input.title)
        .bind(input.motion_text)
        .bind(proposed_by)
        .bind(input.effective_date)
        .bind(input.notes)
        .fetch_one(executor)
        .await
    }

    /// Get a motion, keyed through the root meeting's organization.
    pub async fn get_motion<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingMotion>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMotion>(
            r#"
            SELECT mm.* FROM meeting_motions mm
            WHERE mm.id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = mm.meeting_id AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get motion detail with votes, keyed through the root meeting's
    /// organization. Org validation happens on the root motion fetch.
    pub async fn get_motion_detail(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MotionDetail>, sqlx::Error> {
        let motion = match self.get_motion(&mut *conn, id, org_id).await? {
            Some(m) => m,
            None => return Ok(None),
        };

        let votes = self.list_motion_votes(&mut *conn, id, org_id).await?;

        let vote_summary = VoteSummary {
            total_votes: votes.len() as i32,
            in_favor: motion.votes_in_favor,
            opposed: motion.votes_opposed,
            abstain: motion.votes_abstain,
            recused: motion.votes_recused,
            quorum_met: true, // Would need meeting context for real check
            passed: motion.status == MotionStatus::Passed,
        };

        Ok(Some(MotionDetail {
            motion,
            votes,
            vote_summary,
        }))
    }

    /// List motions for a meeting, keyed through the root meeting's
    /// organization.
    pub async fn list_meeting_motions<'e, E>(
        &self,
        executor: E,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MeetingMotion>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMotion>(
            r#"
            SELECT mm.* FROM meeting_motions mm
            WHERE mm.meeting_id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = mm.meeting_id AND bm.organization_id = $2
                )
            ORDER BY mm.created_at
            "#,
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// List motions with filters.
    pub async fn list_motions<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: MotionQuery,
    ) -> Result<Vec<MotionSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).min(100);
        let offset = (page - 1) * per_page;

        sqlx::query_as::<_, MotionSummary>(
            r#"
            SELECT
                mm.id, mm.motion_number, mm.title, mm.status,
                u.name as proposed_by_name,
                mm.votes_in_favor, mm.votes_opposed, mm.votes_abstain,
                mm.created_at
            FROM meeting_motions mm
            JOIN board_meetings bm ON bm.id = mm.meeting_id
            LEFT JOIN users u ON u.id = mm.proposed_by
            WHERE bm.organization_id = $1
                AND ($2::uuid IS NULL OR mm.meeting_id = $2)
                AND ($3::motion_status IS NULL OR mm.status = $3)
                AND ($4::uuid IS NULL OR mm.proposed_by = $4)
            ORDER BY mm.created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(org_id)
        .bind(query.meeting_id)
        .bind(query.status)
        .bind(query.proposed_by)
        .bind(per_page)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Second a motion, keyed through the root meeting's organization.
    pub async fn second_motion<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        seconded_by: Uuid,
    ) -> Result<Option<MeetingMotion>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMotion>(
            r#"
            UPDATE meeting_motions SET
                seconded_by = $2,
                status = 'seconded',
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_motions.meeting_id
                        AND bm.organization_id = $3
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(seconded_by)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Start voting on a motion, keyed through the root meeting's organization.
    pub async fn start_motion_voting<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingMotion>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMotion>(
            r#"
            UPDATE meeting_motions SET
                status = 'voting',
                voting_started_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_motions.meeting_id
                        AND bm.organization_id = $2
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// End voting on a motion and calculate result, keyed through the root
    /// meeting's organization. Returns `None` when the motion does not belong
    /// to `org_id`.
    pub async fn end_motion_voting(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingMotion>, sqlx::Error> {
        // Org validation on the root motion fetch before counting votes.
        if self.get_motion(&mut *conn, id, org_id).await?.is_none() {
            return Ok(None);
        }

        // Count votes
        let vote_counts: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN vote = 'in_favor' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN vote = 'opposed' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN vote = 'abstain' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN vote = 'recused' THEN 1 ELSE 0 END), 0)
            FROM motion_votes
            WHERE motion_id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;

        let (in_favor, opposed, abstain, recused) = vote_counts;
        let status = if in_favor > opposed {
            MotionStatus::Passed
        } else {
            MotionStatus::Failed
        };

        sqlx::query_as::<_, MeetingMotion>(
            r#"
            UPDATE meeting_motions SET
                status = $2,
                votes_in_favor = $3,
                votes_opposed = $4,
                votes_abstain = $5,
                votes_recused = $6,
                voting_ended_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_motions.meeting_id
                        AND bm.organization_id = $7
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(in_favor as i32)
        .bind(opposed as i32)
        .bind(abstain as i32)
        .bind(recused as i32)
        .bind(org_id)
        .fetch_optional(&mut *conn)
        .await
    }

    /// Update a motion, keyed through the root meeting's organization.
    pub async fn update_motion<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        input: UpdateMotion,
    ) -> Result<Option<MeetingMotion>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMotion>(
            r#"
            UPDATE meeting_motions SET
                title = COALESCE($2, title),
                motion_text = COALESCE($3, motion_text),
                status = COALESCE($4, status),
                resolution_text = COALESCE($5, resolution_text),
                effective_date = COALESCE($6, effective_date),
                notes = COALESCE($7, notes),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_motions.meeting_id
                        AND bm.organization_id = $8
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.title)
        .bind(input.motion_text)
        .bind(input.status)
        .bind(input.resolution_text)
        .bind(input.effective_date)
        .bind(input.notes)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Delete a motion, keyed through the root meeting's organization.
    pub async fn delete_motion<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM meeting_motions
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_motions.meeting_id
                        AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // VOTES
    // ========================================================================

    /// Cast a vote on a motion.
    ///
    /// Callers must validate that `input.motion_id` belongs to the caller's
    /// organization first (root-motion fetch via [`Self::get_motion`]).
    pub async fn cast_vote<'e, E>(
        &self,
        executor: E,
        board_member_id: Uuid,
        input: CastVote,
    ) -> Result<MotionVote, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MotionVote>(
            r#"
            INSERT INTO motion_votes (motion_id, board_member_id, vote, recusal_reason)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (motion_id, board_member_id) DO UPDATE SET
                vote = EXCLUDED.vote,
                recusal_reason = EXCLUDED.recusal_reason,
                voted_at = NOW()
            RETURNING *
            "#,
        )
        .bind(input.motion_id)
        .bind(board_member_id)
        .bind(input.vote)
        .bind(input.recusal_reason)
        .fetch_one(executor)
        .await
    }

    /// List votes for a motion, keyed through the root meeting's organization.
    pub async fn list_motion_votes<'e, E>(
        &self,
        executor: E,
        motion_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MotionVote>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MotionVote>(
            r#"
            SELECT mv.* FROM motion_votes mv
            WHERE mv.motion_id = $1
                AND EXISTS (
                    SELECT 1
                    FROM meeting_motions mm
                    JOIN board_meetings bm ON bm.id = mm.meeting_id
                    WHERE mm.id = mv.motion_id AND bm.organization_id = $2
                )
            "#,
        )
        .bind(motion_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // ATTENDANCE
    // ========================================================================

    /// Record attendance.
    ///
    /// Callers must validate that `input.meeting_id` belongs to the caller's
    /// organization first (root-meeting fetch via [`Self::get_meeting`]).
    pub async fn record_attendance<'e, E>(
        &self,
        executor: E,
        marked_by: Uuid,
        input: RecordAttendance,
    ) -> Result<MeetingAttendance, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAttendance>(
            r#"
            INSERT INTO meeting_attendance (
                meeting_id, board_member_id, user_id, status, attendance_type,
                guest_name, guest_affiliation, notes, marked_by, arrived_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            ON CONFLICT (meeting_id, user_id) DO UPDATE SET
                status = EXCLUDED.status,
                notes = EXCLUDED.notes,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(input.meeting_id)
        .bind(input.board_member_id)
        .bind(input.user_id)
        .bind(input.status)
        .bind(input.attendance_type)
        .bind(input.guest_name)
        .bind(input.guest_affiliation)
        .bind(input.notes)
        .bind(marked_by)
        .fetch_one(executor)
        .await
    }

    /// List attendance for a meeting, keyed through the root meeting's
    /// organization.
    pub async fn list_meeting_attendance<'e, E>(
        &self,
        executor: E,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MeetingAttendance>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingAttendance>(
            r#"
            SELECT ma.* FROM meeting_attendance ma
            WHERE ma.meeting_id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = ma.meeting_id AND bm.organization_id = $2
                )
            ORDER BY ma.arrived_at
            "#,
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Update quorum status for a meeting, scoped to the owning organization.
    pub async fn update_meeting_quorum(
        &self,
        conn: &mut PgConnection,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        // Count present attendees
        let present: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM meeting_attendance
            WHERE meeting_id = $1 AND status IN ('present', 'remote', 'late')
            "#,
        )
        .bind(meeting_id)
        .fetch_one(&mut *conn)
        .await?;

        // Get required quorum (org-keyed root fetch)
        let quorum: (Option<i32>,) = sqlx::query_as(
            "SELECT quorum_required FROM board_meetings WHERE id = $1 AND organization_id = $2",
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        let quorum_met = present.0 >= quorum.0.unwrap_or(1) as i64;

        sqlx::query(
            r#"
            UPDATE board_meetings SET
                quorum_present = $2,
                quorum_met = $3,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $4
            "#,
        )
        .bind(meeting_id)
        .bind(present.0 as i32)
        .bind(quorum_met)
        .bind(org_id)
        .execute(&mut *conn)
        .await?;

        Ok(quorum_met)
    }

    // ========================================================================
    // MINUTES
    // ========================================================================

    /// Create meeting minutes.
    ///
    /// Callers must validate that `input.meeting_id` belongs to the caller's
    /// organization first (root-meeting fetch via [`Self::get_meeting`]).
    pub async fn create_minutes<'e, E>(
        &self,
        executor: E,
        prepared_by: Uuid,
        input: CreateMinutes,
    ) -> Result<MeetingMinutes, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMinutes>(
            r#"
            INSERT INTO meeting_minutes (
                meeting_id, call_to_order, roll_call, approval_of_minutes,
                reports, old_business, new_business, announcements,
                adjournment, full_content, prepared_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(input.meeting_id)
        .bind(input.call_to_order)
        .bind(input.roll_call)
        .bind(input.approval_of_minutes)
        .bind(input.reports)
        .bind(input.old_business)
        .bind(input.new_business)
        .bind(input.announcements)
        .bind(input.adjournment)
        .bind(input.full_content)
        .bind(prepared_by)
        .fetch_one(executor)
        .await
    }

    /// Get minutes for a meeting, keyed through the root meeting's
    /// organization.
    pub async fn get_meeting_minutes<'e, E>(
        &self,
        executor: E,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingMinutes>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMinutes>(
            r#"
            SELECT m.* FROM meeting_minutes m
            WHERE m.meeting_id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = m.meeting_id AND bm.organization_id = $2
                )
            ORDER BY m.version DESC LIMIT 1
            "#,
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Update minutes, keyed through the root meeting's organization.
    pub async fn update_minutes<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        input: UpdateMinutes,
    ) -> Result<Option<MeetingMinutes>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMinutes>(
            r#"
            UPDATE meeting_minutes SET
                status = COALESCE($2, status),
                call_to_order = COALESCE($3, call_to_order),
                roll_call = COALESCE($4, roll_call),
                approval_of_minutes = COALESCE($5, approval_of_minutes),
                reports = COALESCE($6, reports),
                old_business = COALESCE($7, old_business),
                new_business = COALESCE($8, new_business),
                announcements = COALESCE($9, announcements),
                adjournment = COALESCE($10, adjournment),
                full_content = COALESCE($11, full_content),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_minutes.meeting_id
                        AND bm.organization_id = $12
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.status)
        .bind(input.call_to_order)
        .bind(input.roll_call)
        .bind(input.approval_of_minutes)
        .bind(input.reports)
        .bind(input.old_business)
        .bind(input.new_business)
        .bind(input.announcements)
        .bind(input.adjournment)
        .bind(input.full_content)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Approve minutes, keyed through the root meeting's organization.
    pub async fn approve_minutes<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        approved_by: Uuid,
        approval_motion_id: Option<Uuid>,
    ) -> Result<Option<MeetingMinutes>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingMinutes>(
            r#"
            UPDATE meeting_minutes SET
                status = 'approved',
                approved_at = NOW(),
                approved_by = $2,
                approval_motion_id = $3,
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_minutes.meeting_id
                        AND bm.organization_id = $4
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(approved_by)
        .bind(approval_motion_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    // ========================================================================
    // ACTION ITEMS
    // ========================================================================

    /// Create an action item.
    ///
    /// Callers must validate that `input.meeting_id` belongs to the caller's
    /// organization first (root-meeting fetch via [`Self::get_meeting`]).
    pub async fn create_action_item<'e, E>(
        &self,
        executor: E,
        created_by: Uuid,
        input: CreateActionItem,
    ) -> Result<MeetingActionItem, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingActionItem>(
            r#"
            INSERT INTO meeting_action_items (
                meeting_id, agenda_item_id, motion_id, title, description,
                assigned_to, assigned_to_name, due_date, priority, notes, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(input.meeting_id)
        .bind(input.agenda_item_id)
        .bind(input.motion_id)
        .bind(input.title)
        .bind(input.description)
        .bind(input.assigned_to)
        .bind(input.assigned_to_name)
        .bind(input.due_date)
        .bind(input.priority)
        .bind(input.notes)
        .bind(created_by)
        .fetch_one(executor)
        .await
    }

    /// Get an action item, keyed through the root meeting's organization.
    pub async fn get_action_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MeetingActionItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingActionItem>(
            r#"
            SELECT ai.* FROM meeting_action_items ai
            WHERE ai.id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = ai.meeting_id AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List action items for a meeting, keyed through the root meeting's
    /// organization.
    pub async fn list_meeting_action_items<'e, E>(
        &self,
        executor: E,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MeetingActionItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingActionItem>(
            r#"
            SELECT ai.* FROM meeting_action_items ai
            WHERE ai.meeting_id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = ai.meeting_id AND bm.organization_id = $2
                )
            ORDER BY ai.created_at
            "#,
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// List action items with filters.
    pub async fn list_action_items<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ActionItemQuery,
    ) -> Result<Vec<ActionItemSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).min(100);
        let offset = (page - 1) * per_page;

        sqlx::query_as::<_, ActionItemSummary>(
            r#"
            SELECT
                ai.id, ai.title, ai.status, ai.priority, ai.assigned_to_name,
                ai.due_date,
                CASE WHEN ai.due_date < CURRENT_DATE AND ai.status NOT IN ('completed', 'cancelled')
                     THEN true ELSE false END as is_overdue
            FROM meeting_action_items ai
            JOIN board_meetings bm ON bm.id = ai.meeting_id
            WHERE bm.organization_id = $1
                AND ($2::uuid IS NULL OR ai.meeting_id = $2)
                AND ($3::uuid IS NULL OR ai.assigned_to = $3)
                AND ($4::text IS NULL OR ai.status = $4)
                AND ($5::bool IS NULL OR ($5 = true AND ai.due_date < CURRENT_DATE AND ai.status NOT IN ('completed', 'cancelled')))
            ORDER BY ai.due_date ASC NULLS LAST
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(query.meeting_id)
        .bind(query.assigned_to)
        .bind(query.status)
        .bind(query.overdue_only)
        .bind(per_page)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an action item, keyed through the root meeting's organization.
    pub async fn update_action_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        input: UpdateActionItem,
    ) -> Result<Option<MeetingActionItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingActionItem>(
            r#"
            UPDATE meeting_action_items SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                assigned_to = COALESCE($4, assigned_to),
                assigned_to_name = COALESCE($5, assigned_to_name),
                due_date = COALESCE($6, due_date),
                status = COALESCE($7, status),
                priority = COALESCE($8, priority),
                notes = COALESCE($9, notes),
                completion_notes = COALESCE($10, completion_notes),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_action_items.meeting_id
                        AND bm.organization_id = $11
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(input.title)
        .bind(input.description)
        .bind(input.assigned_to)
        .bind(input.assigned_to_name)
        .bind(input.due_date)
        .bind(input.status)
        .bind(input.priority)
        .bind(input.notes)
        .bind(input.completion_notes)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Complete an action item, keyed through the root meeting's organization.
    pub async fn complete_action_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        completion_notes: Option<String>,
    ) -> Result<Option<MeetingActionItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingActionItem>(
            r#"
            UPDATE meeting_action_items SET
                status = 'completed',
                completed_at = NOW(),
                completion_notes = COALESCE($2, completion_notes),
                updated_at = NOW()
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_action_items.meeting_id
                        AND bm.organization_id = $3
                )
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(completion_notes)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Delete an action item, keyed through the root meeting's organization.
    pub async fn delete_action_item<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM meeting_action_items
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_action_items.meeting_id
                        AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // DOCUMENTS
    // ========================================================================

    /// Upload a meeting document.
    ///
    /// Callers must validate that `input.meeting_id` belongs to the caller's
    /// organization first (root-meeting fetch via [`Self::get_meeting`]).
    pub async fn upload_document<'e, E>(
        &self,
        executor: E,
        uploaded_by: Uuid,
        input: UploadMeetingDocument,
    ) -> Result<MeetingDocument, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingDocument>(
            r#"
            INSERT INTO meeting_documents (
                meeting_id, document_type, title, description, file_id,
                file_url, file_name, file_size, mime_type, is_public,
                board_only, uploaded_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(input.meeting_id)
        .bind(input.document_type)
        .bind(input.title)
        .bind(input.description)
        .bind(input.file_id)
        .bind(input.file_url)
        .bind(input.file_name)
        .bind(input.file_size)
        .bind(input.mime_type)
        .bind(input.is_public.unwrap_or(false))
        .bind(input.board_only.unwrap_or(true))
        .bind(uploaded_by)
        .fetch_one(executor)
        .await
    }

    /// List documents for a meeting, keyed through the root meeting's
    /// organization.
    pub async fn list_meeting_documents<'e, E>(
        &self,
        executor: E,
        meeting_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MeetingDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingDocument>(
            r#"
            SELECT md.* FROM meeting_documents md
            WHERE md.meeting_id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = md.meeting_id AND bm.organization_id = $2
                )
            ORDER BY md.created_at
            "#,
        )
        .bind(meeting_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Delete a document, keyed through the root meeting's organization.
    pub async fn delete_document<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM meeting_documents
            WHERE id = $1
                AND EXISTS (
                    SELECT 1 FROM board_meetings bm
                    WHERE bm.id = meeting_documents.meeting_id
                        AND bm.organization_id = $2
                )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // DASHBOARD & STATISTICS
    // ========================================================================

    /// Get meeting dashboard data.
    pub async fn get_dashboard(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<MeetingDashboard, sqlx::Error> {
        let upcoming_meetings = sqlx::query_as::<_, MeetingSummary>(
            r#"
            SELECT
                m.id, m.meeting_number, m.title, m.meeting_type, m.status,
                m.scheduled_start, m.scheduled_end, m.location_type, m.quorum_met,
                (SELECT COUNT(*) FROM meeting_agenda_items WHERE meeting_id = m.id) as agenda_item_count,
                (SELECT COUNT(*) FROM meeting_motions WHERE meeting_id = m.id) as motion_count
            FROM board_meetings m
            WHERE m.organization_id = $1
                AND m.status IN ('draft', 'scheduled')
                AND m.scheduled_start >= NOW()
            ORDER BY m.scheduled_start ASC
            LIMIT 5
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        let recent_meetings = sqlx::query_as::<_, MeetingSummary>(
            r#"
            SELECT
                m.id, m.meeting_number, m.title, m.meeting_type, m.status,
                m.scheduled_start, m.scheduled_end, m.location_type, m.quorum_met,
                (SELECT COUNT(*) FROM meeting_agenda_items WHERE meeting_id = m.id) as agenda_item_count,
                (SELECT COUNT(*) FROM meeting_motions WHERE meeting_id = m.id) as motion_count
            FROM board_meetings m
            WHERE m.organization_id = $1
                AND m.status = 'completed'
            ORDER BY m.scheduled_start DESC
            LIMIT 5
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        let open_action_items = sqlx::query_as::<_, ActionItemSummary>(
            r#"
            SELECT
                ai.id, ai.title, ai.status, ai.priority, ai.assigned_to_name,
                ai.due_date,
                CASE WHEN ai.due_date < CURRENT_DATE THEN true ELSE false END as is_overdue
            FROM meeting_action_items ai
            JOIN board_meetings bm ON bm.id = ai.meeting_id
            WHERE bm.organization_id = $1
                AND ai.status NOT IN ('completed', 'cancelled')
            ORDER BY ai.due_date ASC NULLS LAST
            LIMIT 10
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        let pending_minutes: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM board_meetings bm
            LEFT JOIN meeting_minutes mm ON mm.meeting_id = bm.id AND mm.status = 'approved'
            WHERE bm.organization_id = $1
                AND bm.status = 'completed'
                AND mm.id IS NULL
            "#,
        )
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        let board_member_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM board_members WHERE organization_id = $1 AND is_active = true",
        )
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        let statistics = self.get_latest_statistics(&mut *conn, org_id).await?;

        Ok(MeetingDashboard {
            upcoming_meetings,
            recent_meetings,
            open_action_items,
            pending_minutes: pending_minutes.0,
            board_member_count: board_member_count.0,
            statistics,
        })
    }

    /// Get latest statistics for organization.
    pub async fn get_latest_statistics<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Option<MeetingStatistics>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingStatistics>(
            "SELECT * FROM meeting_statistics WHERE organization_id = $1 ORDER BY calculated_at DESC LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get meeting type counts.
    pub async fn get_meeting_type_counts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<MeetingTypeCount>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeetingTypeCount>(
            r#"
            SELECT meeting_type, COUNT(*) as count
            FROM board_meetings
            WHERE organization_id = $1
            GROUP BY meeting_type
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Get motion status counts.
    pub async fn get_motion_status_counts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<MotionStatusCount>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MotionStatusCount>(
            r#"
            SELECT mm.status, COUNT(*) as count
            FROM meeting_motions mm
            JOIN board_meetings bm ON bm.id = mm.meeting_id
            WHERE bm.organization_id = $1
            GROUP BY mm.status
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Get board member attendance history, scoped to the owning organization.
    pub async fn get_member_attendance_history(
        &self,
        conn: &mut PgConnection,
        board_member_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<AttendanceHistory>, sqlx::Error> {
        let member = match self
            .get_board_member(&mut *conn, board_member_id, org_id)
            .await?
        {
            Some(m) => m,
            None => return Ok(None),
        };

        let attendance: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN status = 'present' OR status = 'late' THEN 1 ELSE 0 END), 0) as attended,
                COALESCE(SUM(CASE WHEN status = 'absent' THEN 1 ELSE 0 END), 0) as absent,
                COALESCE(SUM(CASE WHEN status = 'excused' THEN 1 ELSE 0 END), 0) as excused,
                COALESCE(SUM(CASE WHEN status = 'remote' THEN 1 ELSE 0 END), 0) as remote
            FROM meeting_attendance
            WHERE board_member_id = $1
            "#,
        )
        .bind(board_member_id)
        .fetch_one(&mut *conn)
        .await?;

        let (total, attended, absent, excused, remote) = attendance;
        let attendance_rate = if total > 0 {
            rust_decimal::Decimal::from(attended * 100) / rust_decimal::Decimal::from(total)
        } else {
            rust_decimal::Decimal::ZERO
        };

        Ok(Some(AttendanceHistory {
            board_member: member,
            total_meetings: total,
            attended,
            absent,
            excused,
            remote,
            attendance_rate,
        }))
    }
}

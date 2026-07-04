//! Messaging repository (Epic 6, Story 6.5).
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
//! async fn create_message(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateMessageRequest>,
//! ) -> Result<Json<Message>> {
//!     let message = state.messaging_repo.create_message_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(message))
//! }
//! ```

use crate::models::messaging::{
    BlockWithUserInfo, BlockWithUserInfoRow, CreateBlock, CreateMessage, CreateMessageAttachment,
    CreateThread, Message, MessageAttachment, MessageThread, MessageWithSender,
    MessageWithSenderRow, ThreadWithPreview, ThreadWithPreviewRow, UserBlock,
};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for messaging operations.
#[derive(Clone)]
pub struct MessagingRepository {
    pool: DbPool,
}

/// SQL for [`MessagingRepository::count_unread_rls`].
///
/// Hoisted into a `const` so a non-DB unit test can pin the #1771 soft-delete
/// exclusion (`tps.deleted_at IS NULL`) on the normal CI gate without a live
/// Postgres pool (the DB-backed regression test is quarantined under BIT-351).
const COUNT_UNREAD_SQL: &str = r#"
            SELECT COUNT(*)
            FROM messages m
            JOIN message_threads t ON t.id = m.thread_id
            LEFT JOIN thread_participant_state tps
                ON tps.thread_id = t.id AND tps.user_id = $1
            WHERE $1 = ANY(t.participant_ids)
              AND t.organization_id = $2
              AND m.sender_id != $1
              AND m.deleted_at IS NULL
              -- exclude threads this user soft-deleted ("delete for me"); they
              -- vanish from the inbox list, so their unread messages must not
              -- keep the global unread badge stuck non-zero (#1771).
              AND tps.deleted_at IS NULL
              AND (tps.last_read_at IS NULL OR m.created_at > tps.last_read_at)
            "#;

impl MessagingRepository {
    /// Create a new MessagingRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    // ------------------------------------------------------------------------
    // MESSAGE THREAD OPERATIONS (RLS)
    // ------------------------------------------------------------------------

    /// Get or create a thread between two users with RLS context.
    ///
    /// If a thread already exists between the two users, return it.
    /// Otherwise, create a new thread.
    pub async fn get_or_create_thread_rls<'e, E>(
        &self,
        executor: E,
        data: CreateThread,
    ) -> Result<MessageThread, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Require at least 2 participants. N-party group conversations
        // (UC-05.8 / BIT-183) allow more than 2.
        if data.participant_ids.len() < 2 {
            return Err(SqlxError::Protocol(
                "Thread must have at least 2 participants".to_string(),
            ));
        }

        // Sort + de-dupe participant IDs so the canonical thread is uniquely
        // keyed by (organization_id, participant_ids) for any N.
        let mut sorted_ids = data.participant_ids.clone();
        sorted_ids.sort();
        sorted_ids.dedup();

        // Note: For RLS version, we combine check and insert using ON CONFLICT
        // to avoid needing multiple executor calls
        let thread = sqlx::query_as::<_, MessageThread>(
            r#"
            INSERT INTO message_threads (organization_id, participant_ids)
            VALUES ($1, $2)
            ON CONFLICT ON CONSTRAINT message_threads_organization_id_participant_ids_key
            DO UPDATE SET updated_at = message_threads.updated_at
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(&sorted_ids)
        .fetch_one(executor)
        .await?;

        Ok(thread)
    }

    /// Get a thread by ID with RLS context.
    pub async fn get_thread_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<MessageThread>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let thread = sqlx::query_as::<_, MessageThread>(
            r#"
            SELECT * FROM message_threads WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(thread)
    }

    /// List threads for a user with preview info with RLS context.
    ///
    /// Per-participant state (BIT-182): threads the current user has soft-deleted
    /// (`thread_participant_state.deleted_at` set) are always excluded. When
    /// `archived` is `false` only non-archived threads are returned (the default
    /// inbox); when `true` only the user's archived threads are returned (the
    /// "Archived" tab). A thread with no `thread_participant_state` row defaults
    /// to visible + non-archived.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_threads_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        organization_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        search: Option<&str>,
        archived: bool,
    ) -> Result<Vec<ThreadWithPreview>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = limit.unwrap_or(20).min(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query_as::<_, ThreadWithPreviewRow>(
            r#"
            WITH thread_messages AS (
                SELECT DISTINCT ON (m.thread_id)
                    m.thread_id,
                    m.id as message_id,
                    m.content as message_content,
                    m.sender_id as message_sender_id,
                    m.created_at as message_created_at
                FROM messages m
                WHERE m.deleted_at IS NULL
                ORDER BY m.thread_id, m.created_at DESC
            ),
            -- Per-participant unread (#1773 finding-2): count messages newer
            -- than THIS user's read watermark, not the shared messages.read_at
            -- flag, so a group thread's unread is independent per participant.
            unread_counts AS (
                SELECT m.thread_id, COUNT(*) as unread
                FROM messages m
                LEFT JOIN thread_participant_state tps
                    ON tps.thread_id = m.thread_id AND tps.user_id = $1
                WHERE m.sender_id != $1
                  AND m.deleted_at IS NULL
                  AND (tps.last_read_at IS NULL OR m.created_at > tps.last_read_at)
                GROUP BY m.thread_id
            )
            SELECT
                t.id,
                t.organization_id,
                t.participant_ids,
                t.last_message_at,
                t.created_at,
                t.updated_at,
                -- All other participants (everyone except the current user),
                -- aggregated as a JSON array ([BIT-206]). Issue #1008: `users`
                -- has a single `name` column, not first_name/last_name. Map the
                -- full name into firstName and leave lastName empty to preserve
                -- the ParticipantInfo (camelCase) shape.
                p.participants,
                -- Last message
                tm.message_id as last_message_id,
                tm.message_content as last_message_content,
                tm.message_sender_id as last_message_sender_id,
                tm.message_created_at as last_message_created_at,
                -- Unread count
                COALESCE(uc.unread, 0) as unread_count
            FROM message_threads t
            CROSS JOIN LATERAL (
                -- Aggregate (no GROUP BY) always yields exactly one row, so this
                -- never drops a thread — even a degenerate self-only thread
                -- returns an empty participant list rather than disappearing.
                SELECT
                    COALESCE(
                        json_agg(
                            json_build_object(
                                'id', ou.id,
                                'firstName', ou.name,
                                'lastName', '',
                                'email', ou.email
                            ) ORDER BY ou.name
                        ),
                        '[]'::json
                    ) AS participants,
                    string_agg(ou.name, ' ') AS participant_names
                FROM users ou
                WHERE ou.id = ANY(t.participant_ids) AND ou.id != $1
            ) p
            LEFT JOIN thread_messages tm ON tm.thread_id = t.id
            LEFT JOIN unread_counts uc ON uc.thread_id = t.id
            -- Per-participant view state for the current user (BIT-182).
            LEFT JOIN thread_participant_state tps
                ON tps.thread_id = t.id AND tps.user_id = $1
            WHERE $1 = ANY(t.participant_ids)
              AND t.organization_id = $2
              -- Search matches ANY other participant ([BIT-206]).
              AND ($3::text IS NULL OR p.participant_names ILIKE '%'||$3||'%')
              -- Never show threads this user soft-deleted for themselves.
              AND tps.deleted_at IS NULL
              -- Archived tab vs default inbox.
              AND $6 = (tps.archived_at IS NOT NULL)
            ORDER BY t.last_message_at DESC NULLS LAST, t.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .bind(search)
        .bind(limit)
        .bind(offset)
        .bind(archived)
        .fetch_all(executor)
        .await?;

        let threads = rows
            .into_iter()
            .map(|row| row.into_thread_with_preview(user_id))
            .collect();

        Ok(threads)
    }

    /// Count threads for a user with RLS context.
    ///
    /// Applies the same per-participant filters as [`list_threads_rls`]:
    /// soft-deleted threads are excluded and `archived` selects the archived
    /// tab vs the default inbox.
    pub async fn count_threads_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        organization_id: Uuid,
        search: Option<&str>,
        archived: bool,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM message_threads t
            LEFT JOIN thread_participant_state tps
                ON tps.thread_id = t.id AND tps.user_id = $1
            WHERE $1 = ANY(t.participant_ids)
              AND t.organization_id = $2
              -- Search matches ANY other participant ([BIT-206]); mirrors the
              -- string_agg ILIKE filter in list_threads_rls.
              AND ($3::text IS NULL OR EXISTS (
                    SELECT 1 FROM users ou
                    WHERE ou.id = ANY(t.participant_ids)
                      AND ou.id != $1
                      AND ou.name ILIKE '%'||$3||'%'
              ))
              AND tps.deleted_at IS NULL
              AND $4 = (tps.archived_at IS NOT NULL)
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .bind(search)
        .bind(archived)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    /// Check if user is participant in thread with RLS context.
    pub async fn is_participant_rls<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (is_participant,): (bool,) = sqlx::query_as(
            r#"
            SELECT $2 = ANY(participant_ids)
            FROM message_threads
            WHERE id = $1
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(is_participant)
    }

    // ------------------------------------------------------------------------
    // PER-PARTICIPANT THREAD STATE OPERATIONS (RLS) — BIT-182
    //
    // Archive + per-user soft-delete live in `thread_participant_state`, keyed
    // by (thread_id, user_id). All four mutations are scoped to the *current*
    // user's own row, so one participant changing their view never touches the
    // other participant's copy of the thread. The caller must already have
    // verified participation + tenant (mirror `get_thread` in routes/messaging).
    // ------------------------------------------------------------------------

    /// Soft-hide a thread for a single user (per-user delete).
    ///
    /// Upserts the user's `thread_participant_state` row with `deleted_at = NOW()`.
    /// The shared thread, its messages, and the other participant's view are
    /// untouched. Re-deleting an already-hidden thread is idempotent.
    pub async fn hide_thread_for_user<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO thread_participant_state (thread_id, user_id, deleted_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (thread_id, user_id)
            DO UPDATE SET deleted_at = NOW(), updated_at = NOW()
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Un-hide a thread for a single user (clear a previous per-user delete).
    ///
    /// Called when a new inbound message arrives so a thread the user had
    /// deleted re-appears in their list. No-op when no row / not deleted.
    pub async fn unhide_thread_for_user<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE thread_participant_state
            SET deleted_at = NULL, updated_at = NOW()
            WHERE thread_id = $1 AND user_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Archive a thread for a single user (moves it to their archived tab).
    pub async fn archive_thread_for_user<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO thread_participant_state (thread_id, user_id, archived_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (thread_id, user_id)
            DO UPDATE SET archived_at = NOW(), updated_at = NOW()
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Un-archive a thread for a single user (back to the default inbox).
    pub async fn unarchive_thread_for_user<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE thread_participant_state
            SET archived_at = NULL, updated_at = NOW()
            WHERE thread_id = $1 AND user_id = $2 AND archived_at IS NOT NULL
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // MESSAGE OPERATIONS (RLS)
    // ------------------------------------------------------------------------

    /// Create a new message with RLS context.
    pub async fn create_message_rls<'e, E>(
        &self,
        executor: E,
        data: CreateMessage,
    ) -> Result<Message, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (thread_id, sender_id, content)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(data.thread_id)
        .bind(data.sender_id)
        .bind(&data.content)
        .fetch_one(executor)
        .await?;

        Ok(message)
    }

    /// Get messages for a thread with sender info with RLS context.
    pub async fn get_thread_messages_rls<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<MessageWithSender>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query_as::<_, MessageWithSenderRow>(
            r#"
            SELECT
                m.id,
                m.thread_id,
                m.sender_id,
                m.content,
                m.read_at,
                m.deleted_at,
                m.created_at,
                u.name as sender_first_name, -- #1008: users has only `name`
                '' as sender_last_name,
                u.email as sender_email
            FROM messages m
            JOIN users u ON u.id = m.sender_id
            WHERE m.thread_id = $1
            ORDER BY m.created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(thread_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        let messages = rows.into_iter().map(MessageWithSender::from).collect();

        Ok(messages)
    }

    /// Count messages in a thread with RLS context.
    pub async fn count_thread_messages_rls<'e, E>(
        &self,
        executor: E,
        thread_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM messages
            WHERE thread_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(thread_id)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    /// Mark all messages in a thread as read for a user with RLS context.
    ///
    /// Two effects, both keyed to `reader_id` only (never the other
    /// participants):
    ///  1. stamps `messages.read_at` on this thread's still-unread inbound
    ///     messages — the per-message read receipt (unchanged behaviour);
    ///  2. advances this user's per-participant read watermark
    ///     (`thread_participant_state.last_read_at`), which is what
    ///     [`count_unread_rls`] / the inbox `unread_counts` now derive unread
    ///     from. The watermark is per-(thread, user), so one participant reading
    ///     a group thread no longer zeroes everyone else's unread count
    ///     (#1773 finding-2). The upsert satisfies the table's owner-isolation
    ///     RLS policy because `reader_id == app.current_user_id`.
    ///
    /// Takes `&mut PgConnection` (not a generic executor) so both statements run
    /// on the same RLS-scoped connection.
    pub async fn mark_thread_read_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        thread_id: Uuid,
        reader_id: Uuid,
    ) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE messages
            SET read_at = NOW(), updated_at = NOW()
            WHERE thread_id = $1
              AND sender_id != $2
              AND read_at IS NULL
              AND deleted_at IS NULL
            "#,
        )
        .bind(thread_id)
        .bind(reader_id)
        .execute(&mut *conn)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO thread_participant_state (thread_id, user_id, last_read_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (thread_id, user_id)
            DO UPDATE SET last_read_at = NOW(), updated_at = NOW()
            "#,
        )
        .bind(thread_id)
        .bind(reader_id)
        .execute(&mut *conn)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Count unread messages for a user across all threads with RLS context.
    ///
    /// Unread is derived from the caller's per-participant read watermark
    /// (`thread_participant_state.last_read_at`), not the shared
    /// `messages.read_at` flag: a message counts when it was created after the
    /// caller's watermark (or the caller has no watermark row yet). This makes
    /// group-thread unread counts independent per participant (#1773 finding-2)
    /// — one member reading no longer clears everyone's badge.
    ///
    /// Note (#1771): this LEFT JOIN to `thread_participant_state` is also where
    /// the per-participant soft-delete exclusion lives — `tps.deleted_at IS NULL`
    /// is inline in the query below, so threads a user soft-deleted ("delete for
    /// me") no longer keep their global unread badge stuck non-zero. The query
    /// SQL is hoisted into [`COUNT_UNREAD_SQL`] so a non-DB unit test can pin that
    /// predicate on the normal CI gate (the DB-backed regression test is
    /// quarantined under BIT-351).
    pub async fn count_unread_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (count,): (i64,) = sqlx::query_as(COUNT_UNREAD_SQL)
            .bind(user_id)
            .bind(organization_id)
            .fetch_one(executor)
            .await?;

        Ok(count)
    }

    /// Soft delete a message with RLS context.
    pub async fn delete_message_rls<'e, E>(
        &self,
        executor: E,
        message_id: Uuid,
        deleted_by: Uuid,
    ) -> Result<Message, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let message = sqlx::query_as::<_, Message>(
            r#"
            UPDATE messages
            SET deleted_at = NOW(),
                deleted_by = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(message_id)
        .bind(deleted_by)
        .fetch_one(executor)
        .await?;

        Ok(message)
    }

    // ------------------------------------------------------------------------
    // USER BLOCK OPERATIONS (RLS)
    // ------------------------------------------------------------------------

    /// Block a user with RLS context.
    pub async fn block_user_rls<'e, E>(
        &self,
        executor: E,
        data: CreateBlock,
    ) -> Result<UserBlock, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Use ON CONFLICT to handle already blocked case
        let block = sqlx::query_as::<_, UserBlock>(
            r#"
            INSERT INTO user_blocks (blocker_id, blocked_id, organization_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (blocker_id, blocked_id) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(data.blocker_id)
        .bind(data.blocked_id)
        .bind(data.organization_id)
        .fetch_optional(executor)
        .await?;

        block.ok_or_else(|| SqlxError::Protocol("User is already blocked".to_string()))
    }

    /// Unblock a user with RLS context.
    pub async fn unblock_user_rls<'e, E>(
        &self,
        executor: E,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            DELETE FROM user_blocks
            WHERE blocker_id = $1 AND blocked_id = $2
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Get a specific block with RLS context.
    pub async fn get_block_rls<'e, E>(
        &self,
        executor: E,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<Option<UserBlock>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let block = sqlx::query_as::<_, UserBlock>(
            r#"
            SELECT * FROM user_blocks
            WHERE blocker_id = $1 AND blocked_id = $2
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .fetch_optional(executor)
        .await?;

        Ok(block)
    }

    /// Check if either user has blocked the other with RLS context.
    pub async fn is_blocked_rls<'e, E>(
        &self,
        executor: E,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (exists,): (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_blocks
                WHERE (blocker_id = $1 AND blocked_id = $2)
                   OR (blocker_id = $2 AND blocked_id = $1)
            )
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_one(executor)
        .await?;

        Ok(exists)
    }

    /// Set-based block check for N-party thread creation (#1776).
    ///
    /// Returns the subset of `candidates` that have either blocked `caller` or
    /// been blocked by `caller`, in a single query. Replaces the per-recipient
    /// `is_blocked_rls` loop in `start_thread`, keeping that handler at a
    /// constant number of round-trips regardless of participant count (matching
    /// the already-set-based existence and org-membership checks alongside it).
    pub async fn blocked_among_rls<'e, E>(
        &self,
        executor: E,
        caller: Uuid,
        candidates: &[Uuid],
    ) -> Result<Vec<Uuid>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT blocked_id AS other_id FROM user_blocks
              WHERE blocker_id = $1 AND blocked_id = ANY($2)
            UNION
            SELECT blocker_id AS other_id FROM user_blocks
              WHERE blocked_id = $1 AND blocker_id = ANY($2)
            "#,
        )
        .bind(caller)
        .bind(candidates)
        .fetch_all(executor)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// List blocked users with their info with RLS context.
    pub async fn list_blocked_users_rls<'e, E>(
        &self,
        executor: E,
        blocker_id: Uuid,
    ) -> Result<Vec<BlockWithUserInfo>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query_as::<_, BlockWithUserInfoRow>(
            r#"
            SELECT
                b.id,
                b.blocker_id,
                b.blocked_id,
                b.created_at,
                u.name as blocked_first_name, -- #1008: users has only `name`
                '' as blocked_last_name,
                u.email as blocked_email
            FROM user_blocks b
            JOIN users u ON u.id = b.blocked_id
            WHERE b.blocker_id = $1
            ORDER BY b.created_at DESC
            "#,
        )
        .bind(blocker_id)
        .fetch_all(executor)
        .await?;

        let blocks = rows.into_iter().map(BlockWithUserInfo::from).collect();

        Ok(blocks)
    }

    /// Count blocked users with RLS context.
    pub async fn count_blocked_users_rls<'e, E>(
        &self,
        executor: E,
        blocker_id: Uuid,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM user_blocks WHERE blocker_id = $1
            "#,
        )
        .bind(blocker_id)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    // ------------------------------------------------------------------------
    // MESSAGE THREAD OPERATIONS (Legacy)
    // ------------------------------------------------------------------------

    /// Get or create a thread between two users.
    ///
    /// If a thread already exists between the two users, return it.
    /// Otherwise, create a new thread.
    ///
    /// **Deprecated**: Use `get_or_create_thread_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_or_create_thread_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_or_create_thread(
        &self,
        data: CreateThread,
    ) -> Result<MessageThread, SqlxError> {
        // Ensure exactly 2 participants
        if data.participant_ids.len() != 2 {
            return Err(SqlxError::Protocol(
                "Thread must have exactly 2 participants".to_string(),
            ));
        }

        // Sort participant IDs for consistent lookup
        let mut sorted_ids = data.participant_ids.clone();
        sorted_ids.sort();

        // Check if thread already exists
        let existing = sqlx::query_as::<_, MessageThread>(
            r#"
            SELECT * FROM message_threads
            WHERE organization_id = $1
              AND participant_ids @> $2::uuid[]
              AND participant_ids <@ $2::uuid[]
            "#,
        )
        .bind(data.organization_id)
        .bind(&sorted_ids)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(thread) = existing {
            return Ok(thread);
        }

        // Create new thread
        let thread = sqlx::query_as::<_, MessageThread>(
            r#"
            INSERT INTO message_threads (organization_id, participant_ids)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(&sorted_ids)
        .fetch_one(&self.pool)
        .await?;

        Ok(thread)
    }

    /// Get a thread by ID.
    ///
    /// **Deprecated**: Use `get_thread_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_thread_rls with RlsConnection instead"
    )]
    pub async fn get_thread(&self, id: Uuid) -> Result<Option<MessageThread>, SqlxError> {
        self.get_thread_rls(&self.pool, id).await
    }

    /// List threads for a user with preview info.
    ///
    /// **Deprecated**: Use `list_threads_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_threads_rls with RlsConnection instead"
    )]
    pub async fn list_threads(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ThreadWithPreview>, SqlxError> {
        self.list_threads_rls(
            &self.pool,
            user_id,
            organization_id,
            limit,
            offset,
            None,
            false,
        )
        .await
    }

    /// Count threads for a user.
    ///
    /// **Deprecated**: Use `count_threads_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_threads_rls with RlsConnection instead"
    )]
    pub async fn count_threads(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<i64, SqlxError> {
        self.count_threads_rls(&self.pool, user_id, organization_id, None, false)
            .await
    }

    /// Check if user is participant in thread.
    ///
    /// **Deprecated**: Use `is_participant_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use is_participant_rls with RlsConnection instead"
    )]
    pub async fn is_participant(&self, thread_id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        self.is_participant_rls(&self.pool, thread_id, user_id)
            .await
    }

    // ------------------------------------------------------------------------
    // MESSAGE OPERATIONS (Legacy)
    // ------------------------------------------------------------------------

    /// Create a new message.
    ///
    /// **Deprecated**: Use `create_message_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_message_rls with RlsConnection instead"
    )]
    pub async fn create_message(&self, data: CreateMessage) -> Result<Message, SqlxError> {
        self.create_message_rls(&self.pool, data).await
    }

    /// Get messages for a thread with sender info.
    ///
    /// **Deprecated**: Use `get_thread_messages_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_thread_messages_rls with RlsConnection instead"
    )]
    pub async fn get_thread_messages(
        &self,
        thread_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<MessageWithSender>, SqlxError> {
        self.get_thread_messages_rls(&self.pool, thread_id, limit, offset)
            .await
    }

    /// Count messages in a thread.
    ///
    /// **Deprecated**: Use `count_thread_messages_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_thread_messages_rls with RlsConnection instead"
    )]
    pub async fn count_thread_messages(&self, thread_id: Uuid) -> Result<i64, SqlxError> {
        self.count_thread_messages_rls(&self.pool, thread_id).await
    }

    /// Mark all messages in a thread as read for a user.
    ///
    /// **Deprecated**: Use `mark_thread_read_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use mark_thread_read_rls with RlsConnection instead"
    )]
    pub async fn mark_thread_read(
        &self,
        thread_id: Uuid,
        reader_id: Uuid,
    ) -> Result<i64, SqlxError> {
        // `mark_thread_read_rls` now runs two statements (read receipt + read
        // watermark upsert) on one connection, so acquire a pooled connection
        // and pass it through. (This path is deprecated and non-RLS-scoped.)
        let mut conn = self.pool.acquire().await?;
        self.mark_thread_read_rls(&mut conn, thread_id, reader_id)
            .await
    }

    /// Count unread messages for a user across all threads.
    ///
    /// **Deprecated**: Use `count_unread_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_unread_rls with RlsConnection instead"
    )]
    pub async fn count_unread(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<i64, SqlxError> {
        self.count_unread_rls(&self.pool, user_id, organization_id)
            .await
    }

    /// Soft delete a message.
    ///
    /// **Deprecated**: Use `delete_message_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use delete_message_rls with RlsConnection instead"
    )]
    pub async fn delete_message(
        &self,
        message_id: Uuid,
        deleted_by: Uuid,
    ) -> Result<Message, SqlxError> {
        self.delete_message_rls(&self.pool, message_id, deleted_by)
            .await
    }

    // ------------------------------------------------------------------------
    // USER BLOCK OPERATIONS (Legacy)
    // ------------------------------------------------------------------------

    /// Block a user.
    ///
    /// **Deprecated**: Use `block_user_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use block_user_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn block_user(&self, data: CreateBlock) -> Result<UserBlock, SqlxError> {
        // Check if already blocked
        let existing = self.get_block(data.blocker_id, data.blocked_id).await?;
        if existing.is_some() {
            return Err(SqlxError::Protocol("User is already blocked".to_string()));
        }

        let block = sqlx::query_as::<_, UserBlock>(
            r#"
            INSERT INTO user_blocks (blocker_id, blocked_id, organization_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(data.blocker_id)
        .bind(data.blocked_id)
        .bind(data.organization_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(block)
    }

    /// Unblock a user.
    ///
    /// **Deprecated**: Use `unblock_user_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use unblock_user_rls with RlsConnection instead"
    )]
    pub async fn unblock_user(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<(), SqlxError> {
        self.unblock_user_rls(&self.pool, blocker_id, blocked_id)
            .await
    }

    /// Get a specific block.
    ///
    /// **Deprecated**: Use `get_block_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_block_rls with RlsConnection instead"
    )]
    pub async fn get_block(
        &self,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<Option<UserBlock>, SqlxError> {
        self.get_block_rls(&self.pool, blocker_id, blocked_id).await
    }

    /// Check if either user has blocked the other.
    ///
    /// **Deprecated**: Use `is_blocked_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use is_blocked_rls with RlsConnection instead"
    )]
    pub async fn is_blocked(&self, user_a: Uuid, user_b: Uuid) -> Result<bool, SqlxError> {
        self.is_blocked_rls(&self.pool, user_a, user_b).await
    }

    /// List blocked users with their info.
    ///
    /// **Deprecated**: Use `list_blocked_users_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_blocked_users_rls with RlsConnection instead"
    )]
    pub async fn list_blocked_users(
        &self,
        blocker_id: Uuid,
    ) -> Result<Vec<BlockWithUserInfo>, SqlxError> {
        self.list_blocked_users_rls(&self.pool, blocker_id).await
    }

    /// Count blocked users.
    ///
    /// **Deprecated**: Use `count_blocked_users_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use count_blocked_users_rls with RlsConnection instead"
    )]
    pub async fn count_blocked_users(&self, blocker_id: Uuid) -> Result<i64, SqlxError> {
        self.count_blocked_users_rls(&self.pool, blocker_id).await
    }

    // ------------------------------------------------------------------------
    // MESSAGE ATTACHMENT OPERATIONS (RLS) — UC-05.9 / BIT-184
    // ------------------------------------------------------------------------

    /// Link an uploaded S3 object to a message.
    ///
    /// The caller is responsible for verifying (in the handler) that the
    /// message belongs to a thread the caller participates in and that the
    /// caller is the message sender. RLS (`message_attachments_participant_isolation`)
    /// is the defense-in-depth backstop.
    pub async fn add_attachment_rls<'e, E>(
        &self,
        executor: E,
        data: CreateMessageAttachment,
    ) -> Result<MessageAttachment, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let attachment = sqlx::query_as::<_, MessageAttachment>(
            r#"
            INSERT INTO message_attachments (message_id, file_key, file_name, file_type, file_size)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.message_id)
        .bind(&data.file_key)
        .bind(&data.file_name)
        .bind(&data.file_type)
        .bind(data.file_size)
        .fetch_one(executor)
        .await?;

        Ok(attachment)
    }

    /// List attachments for a single message, oldest first.
    pub async fn get_message_attachments_rls<'e, E>(
        &self,
        executor: E,
        message_id: Uuid,
    ) -> Result<Vec<MessageAttachment>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let attachments = sqlx::query_as::<_, MessageAttachment>(
            r#"
            SELECT * FROM message_attachments
            WHERE message_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(message_id)
        .fetch_all(executor)
        .await?;

        Ok(attachments)
    }

    /// Fetch a single attachment by id together with its owning thread id, so a
    /// download handler can re-verify thread participation/tenant before minting
    /// a presigned URL. Returns `None` when the attachment does not exist (or is
    /// invisible under RLS).
    pub async fn get_attachment_with_thread_rls<'e, E>(
        &self,
        executor: E,
        attachment_id: Uuid,
    ) -> Result<Option<(MessageAttachment, Uuid)>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                i64,
                chrono::DateTime<chrono::Utc>,
                Uuid,
            ),
        >(
            r#"
            SELECT a.id, a.message_id, a.file_key, a.file_name, a.file_type, a.file_size,
                   a.created_at, m.thread_id
            FROM message_attachments a
            JOIN messages m ON m.id = a.message_id
            WHERE a.id = $1
            "#,
        )
        .bind(attachment_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(
            |(id, message_id, file_key, file_name, file_type, file_size, created_at, thread_id)| {
                (
                    MessageAttachment {
                        id,
                        message_id,
                        file_key,
                        file_name,
                        file_type,
                        file_size,
                        created_at,
                    },
                    thread_id,
                )
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::COUNT_UNREAD_SQL;

    /// Non-DB guard for the #1771 fix (PR #1993): the unread-count query must
    /// exclude threads a user soft-deleted ("delete for me") via the
    /// per-participant `tps.deleted_at IS NULL` predicate on the
    /// `thread_participant_state` join. The DB-backed regression test
    /// (`soft_deleted_thread_excluded_from_unread_count`) is `#[ignore]`d under
    /// the BIT-351 quarantine, so this plain `#[test]` — mirroring the
    /// catalog-metadata guard style — is the executing guard on the normal CI
    /// gate. It runs without a live Postgres pool.
    #[test]
    fn count_unread_sql_excludes_soft_deleted_threads() {
        assert!(
            COUNT_UNREAD_SQL.contains("tps.deleted_at IS NULL"),
            "count_unread_rls SQL must keep the per-participant soft-delete \
             exclusion `tps.deleted_at IS NULL` (#1771 / PR #1993); without it a \
             thread a user hid from their inbox leaves its unread messages stuck \
             on the global unread badge with no thread visible to clear them"
        );
    }
}

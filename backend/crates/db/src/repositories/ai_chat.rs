//! AI Chat repository (Epic 13, Story 13.1).
//!
//! # RLS Integration (PAP-80 / PAP-67)
//!
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the three AI-chat tables this repo touches —
//! `ai_chat_sessions`, `ai_chat_messages`, and `ai_training_feedback`. Under
//! `FORCE` the api-server's owner connection is no longer exempt, so a query
//! issued on a connection WITHOUT `app.current_org_id` set collapses to deny-all
//! (own-org reads return empty, writes fail the policy `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `vendor.rs` / `sensor.rs` precedent.
//!
//! Single-statement methods take a generic `E: Executor`. `add_message` writes
//! two statements (the message INSERT and the parent-session `last_message_at`
//! UPDATE) that must run on the SAME RLS-scoped connection, so it takes a
//! `&mut PgConnection` and reborrows.
//!
//! The explicit `organization_id = $N` SQL filters are retained as
//! defense-in-depth; handlers pass `rls.tenant_id()` (the org the caller was
//! validated against), so the SQL filter and the RLS context can never disagree.

use crate::models::{
    AiChatMessage, AiChatSession, AiTrainingFeedback, ChatSessionSummary, CreateChatSession,
    ProvideFeedback,
};
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

/// Repository for AI chat operations.
///
/// Deliberately a zero-sized type: it holds no pool so it cannot issue an
/// un-scoped (deny-all under `FORCE`) query. All queries run on a context-set
/// connection supplied by the handler's `RlsConnection`.
#[derive(Clone)]
pub struct AiChatRepository;

impl AiChatRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    /// Create a new chat session.
    ///
    /// `organization_id` and `user_id` are passed explicitly by the caller and
    /// must originate from the verified request principal, never from client
    /// input in `data`.
    pub async fn create_session<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        data: CreateChatSession,
    ) -> Result<AiChatSession, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO ai_chat_sessions (organization_id, user_id, title, context)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(data.title)
        .bind(sqlx::types::Json(data.context.unwrap_or_default()))
        .fetch_one(executor)
        .await
    }

    /// Get session by ID — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// Returns `None` for both "not found" and "belongs to another tenant"
    /// to prevent cross-tenant ID enumeration.
    pub async fn find_session_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<AiChatSession>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM ai_chat_sessions WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .fetch_optional(executor)
            .await
    }

    /// List user's chat sessions.
    pub async fn list_user_sessions<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatSessionSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                s.id,
                s.title,
                COUNT(m.id) as message_count,
                s.last_message_at,
                s.created_at
            FROM ai_chat_sessions s
            LEFT JOIN ai_chat_messages m ON m.session_id = s.id
            WHERE s.user_id = $1
            GROUP BY s.id
            ORDER BY s.updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Add a message to a session.
    ///
    /// Takes a `&mut PgConnection` (rather than a generic by-value executor) so
    /// the message INSERT and the parent-session `last_message_at` UPDATE both
    /// run on the SAME RLS-scoped connection, keeping `app.current_org_id` in
    /// scope for the second statement's policy check.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_message(
        &self,
        executor: &mut sqlx::PgConnection,
        session_id: Uuid,
        role: &str,
        content: &str,
        confidence: Option<f64>,
        sources: Vec<serde_json::Value>,
        escalated: bool,
        escalation_reason: Option<&str>,
        tokens_used: Option<i32>,
        latency_ms: Option<i32>,
    ) -> Result<AiChatMessage, sqlx::Error> {
        let message: AiChatMessage = sqlx::query_as(
            r#"
            INSERT INTO ai_chat_messages
                (session_id, role, content, confidence, sources, escalated, escalation_reason, tokens_used, latency_ms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(confidence)
        .bind(sqlx::types::Json(sources))
        .bind(escalated)
        .bind(escalation_reason)
        .bind(tokens_used)
        .bind(latency_ms)
        .fetch_one(&mut *executor)
        .await?;

        // Update session last_message_at
        sqlx::query(
            "UPDATE ai_chat_sessions SET last_message_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(session_id)
        .execute(&mut *executor)
        .await?;

        Ok(message)
    }

    /// Get messages for a session — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// The JOIN ensures a caller in org B cannot enumerate messages from a
    /// session owned by org A.
    pub async fn list_session_messages<'e, E>(
        &self,
        executor: E,
        session_id: Uuid,
        org_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AiChatMessage>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT m.* FROM ai_chat_messages m
            JOIN ai_chat_sessions s ON s.id = m.session_id
            WHERE m.session_id = $1 AND s.organization_id = $2
            ORDER BY m.created_at ASC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(session_id)
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Delete a session and all its messages — tenant-scoped.
    ///
    /// `org_id` must originate from the verified request principal.
    /// Returns `false` for both "not found" and "belongs to another tenant"
    /// to prevent cross-tenant ID enumeration.
    pub async fn delete_session<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM ai_chat_sessions WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update session title.
    pub async fn update_session_title<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        title: &str,
    ) -> Result<AiChatSession, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "UPDATE ai_chat_sessions SET title = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(title)
        .fetch_one(executor)
        .await
    }

    /// Add training feedback for a message.
    /// Record AI training feedback for a message.
    ///
    /// `user_id` is passed explicitly by the caller and must originate from the
    /// verified request principal, never from client input in `data`.
    pub async fn add_feedback<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        data: ProvideFeedback,
    ) -> Result<AiTrainingFeedback, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO ai_training_feedback (message_id, user_id, rating, helpful, feedback_text)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (message_id, user_id) DO UPDATE SET
                rating = EXCLUDED.rating,
                helpful = EXCLUDED.helpful,
                feedback_text = EXCLUDED.feedback_text
            RETURNING *
            "#,
        )
        .bind(data.message_id)
        .bind(user_id)
        .bind(data.rating)
        .bind(data.helpful)
        .bind(data.feedback_text)
        .fetch_one(executor)
        .await
    }

    /// Record AI training feedback for a message — tenant-scoped (issue
    /// #766 / #816).
    ///
    /// `user_id` and `org_id` both originate from the verified request
    /// principal. The `INSERT ... SELECT ... WHERE EXISTS` only writes when
    /// the target message's parent session belongs to the caller's org, so a
    /// caller in org B cannot attach feedback to a message in org A's chat
    /// session (an IDOR write + training-data-poisoning vector). Returns
    /// `Ok(None)` when the message does not exist or belongs to another org.
    pub async fn add_feedback_for_org<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        org_id: Uuid,
        data: ProvideFeedback,
    ) -> Result<Option<AiTrainingFeedback>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO ai_training_feedback (message_id, user_id, rating, helpful, feedback_text)
            SELECT $1, $2, $3, $4, $5
            WHERE EXISTS (
                SELECT 1 FROM ai_chat_messages m
                JOIN ai_chat_sessions s ON s.id = m.session_id
                WHERE m.id = $1 AND s.organization_id = $6
            )
            ON CONFLICT (message_id, user_id) DO UPDATE SET
                rating = EXCLUDED.rating,
                helpful = EXCLUDED.helpful,
                feedback_text = EXCLUDED.feedback_text
            RETURNING *
            "#,
        )
        .bind(data.message_id)
        .bind(user_id)
        .bind(data.rating)
        .bind(data.helpful)
        .bind(data.feedback_text)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get escalated messages for review.
    pub async fn list_escalated_messages<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AiChatMessage>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT m.* FROM ai_chat_messages m
            JOIN ai_chat_sessions s ON s.id = m.session_id
            WHERE s.organization_id = $1 AND m.escalated = TRUE
            ORDER BY m.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }
}

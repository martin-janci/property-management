//! AI Chat repository (Epic 13, Story 13.1).

use crate::models::{
    AiChatMessage, AiChatSession, AiTrainingFeedback, ChatSessionSummary, CreateChatSession,
    ProvideFeedback,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Repository for AI chat operations.
#[derive(Clone)]
pub struct AiChatRepository {
    pool: PgPool,
}

impl AiChatRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new chat session.
    ///
    /// `organization_id` and `user_id` are passed explicitly by the caller and
    /// must originate from the verified request principal, never from client
    /// input in `data`.
    pub async fn create_session(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        data: CreateChatSession,
    ) -> Result<AiChatSession, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await
    }

    /// Get session by ID.
    pub async fn find_session_by_id(&self, id: Uuid) -> Result<Option<AiChatSession>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM ai_chat_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List user's chat sessions.
    pub async fn list_user_sessions(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatSessionSummary>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await
    }

    /// Add a message to a session.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_message(
        &self,
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
        .fetch_one(&self.pool)
        .await?;

        // Update session last_message_at
        sqlx::query(
            "UPDATE ai_chat_sessions SET last_message_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(message)
    }

    /// Get messages for a session.
    pub async fn list_session_messages(
        &self,
        session_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AiChatMessage>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT * FROM ai_chat_messages
            WHERE session_id = $1
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a session and all its messages.
    pub async fn delete_session(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM ai_chat_sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update session title.
    pub async fn update_session_title(
        &self,
        id: Uuid,
        title: &str,
    ) -> Result<AiChatSession, sqlx::Error> {
        sqlx::query_as(
            "UPDATE ai_chat_sessions SET title = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(title)
        .fetch_one(&self.pool)
        .await
    }

    /// Add training feedback for a message.
    /// Record AI training feedback for a message.
    ///
    /// `user_id` is passed explicitly by the caller and must originate from the
    /// verified request principal, never from client input in `data`.
    pub async fn add_feedback(
        &self,
        user_id: Uuid,
        data: ProvideFeedback,
    ) -> Result<AiTrainingFeedback, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await
    }

    /// Get escalated messages for review.
    pub async fn list_escalated_messages(
        &self,
        org_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AiChatMessage>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await
    }
}

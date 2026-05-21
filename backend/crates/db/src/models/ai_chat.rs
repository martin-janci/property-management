//! AI Chat models (Epic 13, Story 13.1).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Chat session with AI assistant.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiChatSession {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub context: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
}

/// Individual chat message.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub confidence: Option<f64>,
    pub sources: sqlx::types::Json<Vec<serde_json::Value>>,
    pub escalated: bool,
    pub escalation_reason: Option<String>,
    pub tokens_used: Option<i32>,
    pub latency_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// User feedback on AI responses.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiTrainingFeedback {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub rating: Option<i32>,
    pub helpful: Option<bool>,
    pub feedback_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Message role constants.
pub mod message_role {
    pub const USER: &str = "user";
    pub const ASSISTANT: &str = "assistant";
    pub const SYSTEM: &str = "system";
    pub const ALL: &[&str] = &[USER, ASSISTANT, SYSTEM];
}

/// Request to create a chat session.
///
/// `organization_id` and `user_id` are intentionally NOT part of this struct:
/// the owning org/user are always derived from the verified `RequestPrincipal`
/// server-side, never trusted from client input (prevents IDOR).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateChatSession {
    pub title: Option<String>,
    pub context: Option<serde_json::Value>,
}

/// Request to send a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendChatMessage {
    pub session_id: Uuid,
    pub content: String,
}

/// AI response with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub confidence: f64,
    pub sources: Vec<AiSource>,
    pub escalated: bool,
    pub escalation_reason: Option<String>,
}

/// Source reference in AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSource {
    pub source_type: String,
    pub source_id: Uuid,
    pub title: String,
    pub snippet: Option<String>,
    pub relevance_score: f64,
}

/// Request to provide feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvideFeedback {
    pub message_id: Uuid,
    pub rating: Option<i32>,
    pub helpful: Option<bool>,
    pub feedback_text: Option<String>,
}

/// Session summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatSessionSummary {
    pub id: Uuid,
    pub title: Option<String>,
    pub message_count: i64,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

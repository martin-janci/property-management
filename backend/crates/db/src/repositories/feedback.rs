//! General in-app feedback repository (#3544).
//!
//! Durable store for the mobile client's general feedback / bug reports
//! (`POST /api/v1/feedback`). See migration `00246_create_app_feedback.sql`
//! for the table shape and the (deliberately RLS-free, user-scoped) design
//! rationale.
//!
//! Uses the runtime `sqlx::query*` API (not the compile-time macros) so the
//! crate builds without a live `DATABASE_URL` — the repo-wide convention
//! documented in `backend/.github/workflows/backend.yml` (sqlx-migrations
//! gate) and mirrored across `repositories/*.rs`.

use crate::DbPool;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for general in-app feedback operations.
#[derive(Clone)]
pub struct FeedbackRepository {
    pool: DbPool,
}

/// A new general feedback submission to persist.
#[derive(Debug, Clone)]
pub struct NewFeedback<'a> {
    /// Authenticated submitter.
    pub user_id: Uuid,
    /// Category: bug | feature | improvement | question | other.
    pub feedback_type: &'a str,
    /// Short summary.
    pub title: &'a str,
    /// Free-text body.
    pub description: &'a str,
    /// Optional reply-to address.
    pub email: Option<&'a str>,
    /// Optional screenshot payload (data URI / object key).
    pub screenshot: Option<&'a str>,
    /// Optional client `deviceInfo` blob.
    pub device_info: Option<JsonValue>,
    /// Optional client `appContext` blob.
    pub app_context: Option<JsonValue>,
    /// Optional active organization/building context (triage hint).
    pub organization_id: Option<Uuid>,
}

impl FeedbackRepository {
    /// Create a new feedback repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Persist a feedback submission and return its generated id.
    pub async fn create_feedback(&self, feedback: NewFeedback<'_>) -> Result<Uuid, SqlxError> {
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO app_feedback (
                user_id, feedback_type, title, description,
                email, screenshot, device_info, app_context, organization_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#,
        )
        .bind(feedback.user_id)
        .bind(feedback.feedback_type)
        .bind(feedback.title)
        .bind(feedback.description)
        .bind(feedback.email)
        .bind(feedback.screenshot)
        .bind(feedback.device_info)
        .bind(feedback.app_context)
        .bind(feedback.organization_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Count a user's feedback submissions (triage / tests).
    pub async fn count_for_user(&self, user_id: Uuid) -> Result<i64, SqlxError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM app_feedback WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}

/// A persisted feedback record (triage reads).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct FeedbackRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub feedback_type: String,
    pub title: String,
    pub description: String,
    pub email: Option<String>,
    pub screenshot: Option<String>,
    pub device_info: Option<JsonValue>,
    pub app_context: Option<JsonValue>,
    pub organization_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

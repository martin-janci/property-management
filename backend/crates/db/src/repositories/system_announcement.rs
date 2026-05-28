//! System Announcement repository (Epic 10B, Story 10B.4).
//!
//! Repository for platform-wide system announcements and scheduled maintenance.

use crate::models::platform_admin::{
    ScheduledMaintenance, SystemAnnouncement, SystemAnnouncementAcknowledgment,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for system announcement operations.
#[derive(Clone)]
pub struct SystemAnnouncementRepository {
    pool: DbPool,
}

/// Active announcement for user.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ActiveAnnouncement {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub severity: String,
    pub start_at: DateTime<Utc>,
    pub end_at: Option<DateTime<Utc>>,
    pub is_dismissible: bool,
    pub requires_acknowledgment: bool,
    /// Whether this user has already acknowledged this announcement
    pub is_acknowledged: bool,
}

impl SystemAnnouncementRepository {
    /// Create a new SystemAnnouncementRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new system announcement.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_announcement(
        &self,
        title: &str,
        message: &str,
        severity: &str,
        start_at: DateTime<Utc>,
        end_at: Option<DateTime<Utc>>,
        is_dismissible: bool,
        requires_acknowledgment: bool,
        created_by: Uuid,
    ) -> Result<SystemAnnouncement, SqlxError> {
        let announcement = sqlx::query_as::<_, SystemAnnouncement>(
            r#"
            INSERT INTO system_announcements (title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by, created_at, updated_at
            "#,
        )
        .bind(title)
        .bind(message)
        .bind(severity)
        .bind(start_at)
        .bind(end_at)
        .bind(is_dismissible)
        .bind(requires_acknowledgment)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(announcement)
    }

    /// Update an existing announcement.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_announcement(
        &self,
        id: Uuid,
        title: Option<&str>,
        message: Option<&str>,
        severity: Option<&str>,
        start_at: Option<DateTime<Utc>>,
        end_at: Option<Option<DateTime<Utc>>>,
        is_dismissible: Option<bool>,
        requires_acknowledgment: Option<bool>,
    ) -> Result<Option<SystemAnnouncement>, SqlxError> {
        let announcement = sqlx::query_as::<_, SystemAnnouncement>(
            r#"
            UPDATE system_announcements
            SET
                title = COALESCE($2, title),
                message = COALESCE($3, message),
                severity = COALESCE($4, severity),
                start_at = COALESCE($5, start_at),
                end_at = COALESCE($6, end_at),
                is_dismissible = COALESCE($7, is_dismissible),
                requires_acknowledgment = COALESCE($8, requires_acknowledgment),
                updated_at = NOW()
            WHERE id = $1 AND is_deleted = false
            RETURNING id, title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(message)
        .bind(severity)
        .bind(start_at)
        .bind(end_at)
        .bind(is_dismissible)
        .bind(requires_acknowledgment)
        .fetch_optional(&self.pool)
        .await?;

        Ok(announcement)
    }

    /// Soft delete an announcement.
    pub async fn delete_announcement(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "UPDATE system_announcements SET is_deleted = true, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get announcement by ID.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<SystemAnnouncement>, SqlxError> {
        let announcement = sqlx::query_as::<_, SystemAnnouncement>(
            r#"
            SELECT id, title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by, created_at, updated_at
            FROM system_announcements
            WHERE id = $1 AND is_deleted = false
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(announcement)
    }

    /// List all announcements (for admin view).
    pub async fn list_all(
        &self,
        include_deleted: bool,
    ) -> Result<Vec<SystemAnnouncement>, SqlxError> {
        let query = if include_deleted {
            r#"
            SELECT id, title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by, created_at, updated_at
            FROM system_announcements
            ORDER BY created_at DESC
            "#
        } else {
            r#"
            SELECT id, title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by, created_at, updated_at
            FROM system_announcements
            WHERE is_deleted = false
            ORDER BY created_at DESC
            "#
        };

        let announcements = sqlx::query_as::<_, SystemAnnouncement>(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(announcements)
    }

    /// Get currently active announcements.
    pub async fn get_active_announcements(&self) -> Result<Vec<SystemAnnouncement>, SqlxError> {
        let announcements = sqlx::query_as::<_, SystemAnnouncement>(
            r#"
            SELECT id, title, message, severity, start_at, end_at, is_dismissible, requires_acknowledgment, created_by, created_at, updated_at
            FROM system_announcements
            WHERE is_deleted = false
              AND start_at <= NOW()
              AND (end_at IS NULL OR end_at > NOW())
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 1
                    WHEN 'warning' THEN 2
                    ELSE 3
                END,
                created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(announcements)
    }

    /// Get active announcements for a specific user (with acknowledgment status).
    pub async fn get_active_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ActiveAnnouncement>, SqlxError> {
        let announcements = sqlx::query_as::<_, ActiveAnnouncement>(
            r#"
            SELECT
                a.id, a.title, a.message, a.severity, a.start_at, a.end_at,
                a.is_dismissible, a.requires_acknowledgment,
                CASE WHEN ack.id IS NOT NULL THEN true ELSE false END as is_acknowledged
            FROM system_announcements a
            LEFT JOIN system_announcement_acknowledgments ack
                ON ack.announcement_id = a.id AND ack.user_id = $1
            WHERE a.is_deleted = false
              AND a.start_at <= NOW()
              AND (a.end_at IS NULL OR a.end_at > NOW())
            ORDER BY
                CASE a.severity
                    WHEN 'critical' THEN 1
                    WHEN 'warning' THEN 2
                    ELSE 3
                END,
                a.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(announcements)
    }

    /// Get unacknowledged critical announcements for a user.
    pub async fn get_unacknowledged_critical(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SystemAnnouncement>, SqlxError> {
        let announcements = sqlx::query_as::<_, SystemAnnouncement>(
            r#"
            SELECT a.id, a.title, a.message, a.severity, a.start_at, a.end_at, a.is_dismissible, a.requires_acknowledgment, a.created_by, a.created_at, a.updated_at
            FROM system_announcements a
            LEFT JOIN system_announcement_acknowledgments ack
                ON ack.announcement_id = a.id AND ack.user_id = $1
            WHERE a.is_deleted = false
              AND a.requires_acknowledgment = true
              AND a.start_at <= NOW()
              AND (a.end_at IS NULL OR a.end_at > NOW())
              AND ack.id IS NULL
            ORDER BY a.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(announcements)
    }

    /// Record user acknowledgment of an announcement.
    pub async fn record_acknowledgment(
        &self,
        announcement_id: Uuid,
        user_id: Uuid,
    ) -> Result<SystemAnnouncementAcknowledgment, SqlxError> {
        let ack = sqlx::query_as::<_, SystemAnnouncementAcknowledgment>(
            r#"
            INSERT INTO system_announcement_acknowledgments (announcement_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (announcement_id, user_id) DO UPDATE SET acknowledged_at = NOW()
            RETURNING id, announcement_id, user_id, acknowledged_at
            "#,
        )
        .bind(announcement_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ack)
    }

    /// Schedule a maintenance window.
    #[allow(clippy::too_many_arguments)]
    pub async fn schedule_maintenance(
        &self,
        title: &str,
        description: Option<&str>,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
        is_read_only_mode: bool,
        announcement_id: Option<Uuid>,
        created_by: Uuid,
    ) -> Result<ScheduledMaintenance, SqlxError> {
        let maintenance = sqlx::query_as::<_, ScheduledMaintenance>(
            r#"
            INSERT INTO scheduled_maintenance (title, description, start_at, end_at, is_read_only_mode, announcement_id, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, title, description, start_at, end_at, is_read_only_mode, announcement_id, created_by, created_at
            "#,
        )
        .bind(title)
        .bind(description)
        .bind(start_at)
        .bind(end_at)
        .bind(is_read_only_mode)
        .bind(announcement_id)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(maintenance)
    }

    /// Get upcoming maintenance windows.
    pub async fn get_upcoming_maintenance(&self) -> Result<Vec<ScheduledMaintenance>, SqlxError> {
        let maintenance = sqlx::query_as::<_, ScheduledMaintenance>(
            r#"
            SELECT id, title, description, start_at, end_at, is_read_only_mode, announcement_id, created_by, created_at
            FROM scheduled_maintenance
            WHERE end_at > NOW()
            ORDER BY start_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(maintenance)
    }

    /// Get currently active maintenance window (if any).
    pub async fn get_active_maintenance(&self) -> Result<Option<ScheduledMaintenance>, SqlxError> {
        let maintenance = sqlx::query_as::<_, ScheduledMaintenance>(
            r#"
            SELECT id, title, description, start_at, end_at, is_read_only_mode, announcement_id, created_by, created_at
            FROM scheduled_maintenance
            WHERE start_at <= NOW() AND end_at > NOW()
            ORDER BY start_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(maintenance)
    }

    /// Delete a scheduled maintenance.
    pub async fn delete_maintenance(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query("DELETE FROM scheduled_maintenance WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_announcement_fields() {
        let announcement = ActiveAnnouncement {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            message: "Test message".to_string(),
            severity: "info".to_string(),
            start_at: Utc::now(),
            end_at: None,
            is_dismissible: true,
            requires_acknowledgment: false,
            is_acknowledged: false,
        };

        assert!(!announcement.is_acknowledged);
        assert!(announcement.is_dismissible);
    }

    #[test]
    fn active_announcement_acknowledged_variant() {
        let announcement = ActiveAnnouncement {
            id: Uuid::new_v4(),
            title: "Critical Outage".to_string(),
            message: "Service disruption in progress.".to_string(),
            severity: "critical".to_string(),
            start_at: Utc::now(),
            end_at: Some(Utc::now() + chrono::Duration::hours(2)),
            is_dismissible: false,
            requires_acknowledgment: true,
            is_acknowledged: true,
        };

        assert!(announcement.is_acknowledged);
        assert!(!announcement.is_dismissible);
        assert!(announcement.requires_acknowledgment);
        assert!(announcement.end_at.is_some());
    }

    #[test]
    fn active_announcement_serde_round_trip() {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let announcement = ActiveAnnouncement {
            id,
            title: "Maintenance".to_string(),
            message: "Scheduled downtime.".to_string(),
            severity: "warning".to_string(),
            start_at: now,
            end_at: None,
            is_dismissible: true,
            requires_acknowledgment: false,
            is_acknowledged: false,
        };

        let json = serde_json::to_value(&announcement).expect("serialize");
        assert_eq!(json["title"], "Maintenance");
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["is_dismissible"], true);
        assert_eq!(json["is_acknowledged"], false);
        assert!(json["end_at"].is_null());
    }
}

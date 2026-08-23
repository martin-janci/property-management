//! Granular notification preferences repository (Epic 8B).
//!
//! Provides database operations for per-event and per-channel notification preferences.

use chrono::{NaiveTime, Utc};
use sqlx::{Error as SqlxError, FromRow, PgPool, Row};
use uuid::Uuid;

use crate::models::{
    CreateHeldNotification, EventNotificationPreference, EventPreferenceWithDetails,
    HeldNotification, NotificationEventCategory, NotificationEventType, NotificationSchedule,
    RoleNotificationDefaults,
};

/// Repository for granular notification preferences.
#[derive(Clone)]
pub struct GranularNotificationRepository {
    pool: PgPool,
}

/// A user eligible for a 24h-inactivity email digest (Story 2B.3, PM #969 gap 1).
/// Produced by [`GranularNotificationRepository::find_inactive_digest_candidates`].
#[derive(Debug, Clone, FromRow)]
pub struct InactiveDigestCandidate {
    pub user_id: Uuid,
    pub email: String,
    pub locale: String,
    /// Total unread notifications across all categories.
    pub notification_count: i64,
    /// Unread counts keyed by `entity_type`, e.g. `{ "fault": 5, "vote": 2 }`.
    pub category_counts: serde_json::Value,
    /// Earliest unread notification group creation time.
    pub period_start: chrono::DateTime<Utc>,
    /// Most recent unread activity time.
    pub period_end: chrono::DateTime<Utc>,
}

/// A persisted digest row whose email has not yet been delivered, joined with
/// the recipient address so it can be (re)sent. Produced by
/// [`GranularNotificationRepository::get_unsent_digests`].
#[derive(Debug, Clone, FromRow)]
pub struct PendingDigestEmail {
    pub digest_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub locale: String,
    pub notification_count: i32,
    pub category_counts: serde_json::Value,
    pub period_start: chrono::DateTime<Utc>,
    pub period_end: chrono::DateTime<Utc>,
}

impl GranularNotificationRepository {
    /// Create a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Event Types (Reference Data)
    // ========================================================================

    /// List all available notification event types.
    pub async fn list_event_types(&self) -> Result<Vec<NotificationEventType>, SqlxError> {
        sqlx::query_as::<_, NotificationEventType>(
            r#"
            SELECT * FROM notification_event_types
            ORDER BY category, event_type
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// List event types by category.
    pub async fn list_event_types_by_category(
        &self,
        category: NotificationEventCategory,
    ) -> Result<Vec<NotificationEventType>, SqlxError> {
        sqlx::query_as::<_, NotificationEventType>(
            r#"
            SELECT * FROM notification_event_types
            WHERE category = $1
            ORDER BY event_type
            "#,
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Event Notification Preferences (Stories 8B.1 & 8B.2)
    // ========================================================================

    /// Get all event preferences for a user with event type details.
    pub async fn get_user_event_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EventPreferenceWithDetails>, SqlxError> {
        let rows = sqlx::query(
            r#"
            SELECT
                et.event_type,
                et.category,
                et.display_name,
                et.description,
                et.is_priority,
                COALESCE(ep.push_enabled, et.default_push) as push_enabled,
                COALESCE(ep.email_enabled, et.default_email) as email_enabled,
                COALESCE(ep.in_app_enabled, et.default_in_app) as in_app_enabled,
                ep.updated_at
            FROM notification_event_types et
            LEFT JOIN event_notification_preferences ep
                ON ep.event_type = et.event_type AND ep.user_id = $1
            ORDER BY et.category, et.event_type
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| EventPreferenceWithDetails {
                event_type: r.get("event_type"),
                category: r.get("category"),
                display_name: r.get("display_name"),
                description: r.get("description"),
                is_priority: r.get("is_priority"),
                push_enabled: r.get("push_enabled"),
                email_enabled: r.get("email_enabled"),
                in_app_enabled: r.get("in_app_enabled"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Get preference for a specific event type.
    pub async fn get_user_event_preference(
        &self,
        user_id: Uuid,
        event_type: &str,
    ) -> Result<Option<EventNotificationPreference>, SqlxError> {
        sqlx::query_as::<_, EventNotificationPreference>(
            r#"
            SELECT * FROM event_notification_preferences
            WHERE user_id = $1 AND event_type = $2
            "#,
        )
        .bind(user_id)
        .bind(event_type)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update or create event preference for a user.
    pub async fn upsert_event_preference(
        &self,
        user_id: Uuid,
        event_type: &str,
        push_enabled: Option<bool>,
        email_enabled: Option<bool>,
        in_app_enabled: Option<bool>,
    ) -> Result<EventNotificationPreference, SqlxError> {
        // Get the category from event types
        let event_type_row =
            sqlx::query("SELECT category FROM notification_event_types WHERE event_type = $1")
                .bind(event_type)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| SqlxError::RowNotFound)?;

        let category: NotificationEventCategory = event_type_row.get("category");

        sqlx::query_as::<_, EventNotificationPreference>(
            r#"
            INSERT INTO event_notification_preferences (
                user_id, event_type, event_category, push_enabled, email_enabled, in_app_enabled
            )
            VALUES ($1, $2, $3, COALESCE($4, true), COALESCE($5, true), COALESCE($6, true))
            ON CONFLICT (user_id, event_type) DO UPDATE SET
                push_enabled = COALESCE($4, event_notification_preferences.push_enabled),
                email_enabled = COALESCE($5, event_notification_preferences.email_enabled),
                in_app_enabled = COALESCE($6, event_notification_preferences.in_app_enabled),
                updated_at = now()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(event_type)
        .bind(category)
        .bind(push_enabled)
        .bind(email_enabled)
        .bind(in_app_enabled)
        .fetch_one(&self.pool)
        .await
    }

    /// Reset all event preferences for a user to defaults.
    pub async fn reset_event_preferences(&self, user_id: Uuid) -> Result<i64, SqlxError> {
        let result = sqlx::query("DELETE FROM event_notification_preferences WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Bulk update event preferences for a category.
    pub async fn update_category_preferences(
        &self,
        user_id: Uuid,
        category: NotificationEventCategory,
        push_enabled: Option<bool>,
        email_enabled: Option<bool>,
        in_app_enabled: Option<bool>,
    ) -> Result<i64, SqlxError> {
        // Get all event types for this category
        let event_types = self.list_event_types_by_category(category).await?;

        let mut updated = 0i64;
        for et in event_types {
            self.upsert_event_preference(
                user_id,
                &et.event_type,
                push_enabled,
                email_enabled,
                in_app_enabled,
            )
            .await?;
            updated += 1;
        }

        Ok(updated)
    }

    // ========================================================================
    // Notification Schedule (Story 8B.3)
    // ========================================================================

    /// Get user's notification schedule.
    pub async fn get_user_schedule(
        &self,
        user_id: Uuid,
    ) -> Result<Option<NotificationSchedule>, SqlxError> {
        sqlx::query_as::<_, NotificationSchedule>(
            "SELECT * FROM notification_schedule WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create or update user's notification schedule.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_schedule(
        &self,
        user_id: Uuid,
        quiet_hours_enabled: Option<bool>,
        quiet_hours_start: Option<NaiveTime>,
        quiet_hours_end: Option<NaiveTime>,
        timezone: Option<&str>,
        weekend_quiet_hours_enabled: Option<bool>,
        weekend_quiet_hours_start: Option<NaiveTime>,
        weekend_quiet_hours_end: Option<NaiveTime>,
        digest_enabled: Option<bool>,
        digest_frequency: Option<&str>,
        digest_time: Option<NaiveTime>,
        digest_day_of_week: Option<i32>,
    ) -> Result<NotificationSchedule, SqlxError> {
        sqlx::query_as::<_, NotificationSchedule>(
            r#"
            INSERT INTO notification_schedule (
                user_id, quiet_hours_enabled, quiet_hours_start, quiet_hours_end, timezone,
                weekend_quiet_hours_enabled, weekend_quiet_hours_start, weekend_quiet_hours_end,
                digest_enabled, digest_frequency, digest_time, digest_day_of_week
            )
            VALUES ($1, COALESCE($2, false), $3, $4, COALESCE($5, 'UTC'),
                    COALESCE($6, false), $7, $8,
                    COALESCE($9, false), $10, $11, $12)
            ON CONFLICT (user_id) DO UPDATE SET
                quiet_hours_enabled = COALESCE($2, notification_schedule.quiet_hours_enabled),
                quiet_hours_start = COALESCE($3, notification_schedule.quiet_hours_start),
                quiet_hours_end = COALESCE($4, notification_schedule.quiet_hours_end),
                timezone = COALESCE($5, notification_schedule.timezone),
                weekend_quiet_hours_enabled = COALESCE($6, notification_schedule.weekend_quiet_hours_enabled),
                weekend_quiet_hours_start = COALESCE($7, notification_schedule.weekend_quiet_hours_start),
                weekend_quiet_hours_end = COALESCE($8, notification_schedule.weekend_quiet_hours_end),
                digest_enabled = COALESCE($9, notification_schedule.digest_enabled),
                digest_frequency = COALESCE($10, notification_schedule.digest_frequency),
                digest_time = COALESCE($11, notification_schedule.digest_time),
                digest_day_of_week = COALESCE($12, notification_schedule.digest_day_of_week),
                updated_at = now()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(quiet_hours_enabled)
        .bind(quiet_hours_start)
        .bind(quiet_hours_end)
        .bind(timezone)
        .bind(weekend_quiet_hours_enabled)
        .bind(weekend_quiet_hours_start)
        .bind(weekend_quiet_hours_end)
        .bind(digest_enabled)
        .bind(digest_frequency)
        .bind(digest_time)
        .bind(digest_day_of_week)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // Held Notifications (Story 8B.3)
    // ========================================================================

    /// Create a held notification.
    pub async fn create_held_notification(
        &self,
        notification: CreateHeldNotification,
    ) -> Result<HeldNotification, SqlxError> {
        sqlx::query_as::<_, HeldNotification>(
            r#"
            INSERT INTO held_notifications (
                user_id, event_type, title, body, data, channels, release_at, is_priority
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(notification.user_id)
        .bind(&notification.event_type)
        .bind(&notification.title)
        .bind(&notification.body)
        .bind(&notification.data)
        .bind(&notification.channels)
        .bind(notification.release_at)
        .bind(notification.is_priority)
        .fetch_one(&self.pool)
        .await
    }

    /// Get pending held notifications for a user.
    pub async fn get_pending_held_notifications(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<HeldNotification>, SqlxError> {
        sqlx::query_as::<_, HeldNotification>(
            r#"
            SELECT * FROM held_notifications
            WHERE user_id = $1 AND released_at IS NULL
            ORDER BY held_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Atomically claim up to `batch_limit` held notifications that are ready
    /// for release, returning the rows this caller now owns (issue #2831).
    ///
    /// The drain worker runs in every api-server process. A plain
    /// `SELECT ... WHERE released_at IS NULL` let every replica pick up the same
    /// due rows and deliver them, so a held notification was delivered once per
    /// replica (double-delivery under >1 replica). This claims each row instead:
    /// a single `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED)
    /// RETURNING *` stamps `claimed_at` and hands back only the rows this
    /// UPDATE won. Concurrent claimers `SKIP LOCKED` past a row another replica
    /// is claiming and, once that claim commits, see a fresh `claimed_at` that
    /// excludes the row for the lease window — so each due row is delivered by
    /// at most one replica.
    ///
    /// `claim_lease_secs` makes the claim self-healing: a claim older than the
    /// lease is treated as unclaimed, so a worker that crashed mid-delivery does
    /// not strand the row — another replica re-claims it once the lease expires.
    ///
    /// Still excludes released and dead-lettered rows (issue #2823): both are
    /// terminal and must never be re-selected.
    pub async fn claim_notifications_to_release(
        &self,
        batch_limit: i64,
        claim_lease_secs: i64,
    ) -> Result<Vec<HeldNotification>, SqlxError> {
        sqlx::query_as::<_, HeldNotification>(
            r#"
            UPDATE held_notifications AS h
            SET claimed_at = now()
            WHERE h.id IN (
                SELECT id FROM held_notifications
                WHERE released_at IS NULL
                  AND dead_lettered_at IS NULL
                  AND release_at <= now()
                  AND (claimed_at IS NULL
                       OR claimed_at <= now() - make_interval(secs => $1))
                ORDER BY release_at
                LIMIT $2
                FOR UPDATE SKIP LOCKED
            )
            RETURNING h.*
            "#,
        )
        .bind(claim_lease_secs as f64)
        .bind(batch_limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Mark held notification as released.
    pub async fn mark_notification_released(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE held_notifications SET released_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Persist progress after a partial-failure drain tick (issue #2823):
    /// record the channels delivered so far and the bumped attempt counter so
    /// the next retry skips already-delivered channels and the retry budget is
    /// enforced. The row stays held (`released_at`/`dead_lettered_at` untouched).
    ///
    /// Clears `claimed_at` (issue #2831): the row was handled this tick but must
    /// be retried, so releasing the claim lets the next tick re-claim it
    /// promptly (from any replica) instead of waiting out the lease. The retry
    /// is safe across replicas because the persisted `delivered_channels` makes
    /// `deliver_held` skip the channels that already succeeded.
    pub async fn record_held_attempt(
        &self,
        id: Uuid,
        delivered_channels: &[String],
        attempts: i32,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            "UPDATE held_notifications SET delivered_channels = $2, attempts = $3, claimed_at = NULL WHERE id = $1",
        )
        .bind(id)
        .bind(delivered_channels)
        .bind(attempts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Give up on a held row that has exhausted its retry budget (issue #2823):
    /// stamp `dead_lettered_at` so it is no longer re-selected for release,
    /// instead of looping forever and re-delivering its healthy channels.
    pub async fn mark_notification_dead_lettered(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE held_notifications SET dead_lettered_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // Role-Based Defaults (Story 8B.4)
    // ========================================================================

    /// Get role notification defaults for an organization.
    pub async fn get_role_defaults(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<RoleNotificationDefaults>, SqlxError> {
        sqlx::query_as::<_, RoleNotificationDefaults>(
            r#"
            SELECT * FROM role_notification_defaults
            WHERE organization_id = $1
            ORDER BY role
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get role defaults for a specific role.
    pub async fn get_role_default(
        &self,
        organization_id: Uuid,
        role: &str,
    ) -> Result<Option<RoleNotificationDefaults>, SqlxError> {
        sqlx::query_as::<_, RoleNotificationDefaults>(
            r#"
            SELECT * FROM role_notification_defaults
            WHERE organization_id = $1 AND role = $2
            "#,
        )
        .bind(organization_id)
        .bind(role)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create or update role notification defaults.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_role_defaults(
        &self,
        organization_id: Uuid,
        role: &str,
        event_preferences: serde_json::Value,
        default_quiet_hours_enabled: Option<bool>,
        default_quiet_hours_start: Option<NaiveTime>,
        default_quiet_hours_end: Option<NaiveTime>,
        created_by: Uuid,
    ) -> Result<RoleNotificationDefaults, SqlxError> {
        sqlx::query_as::<_, RoleNotificationDefaults>(
            r#"
            INSERT INTO role_notification_defaults (
                organization_id, role, event_preferences,
                default_quiet_hours_enabled, default_quiet_hours_start, default_quiet_hours_end,
                created_by
            )
            VALUES ($1, $2, $3, COALESCE($4, false), $5, $6, $7)
            ON CONFLICT (organization_id, role) DO UPDATE SET
                event_preferences = $3,
                default_quiet_hours_enabled = COALESCE($4, role_notification_defaults.default_quiet_hours_enabled),
                default_quiet_hours_start = COALESCE($5, role_notification_defaults.default_quiet_hours_start),
                default_quiet_hours_end = COALESCE($6, role_notification_defaults.default_quiet_hours_end),
                updated_at = now()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(role)
        .bind(&event_preferences)
        .bind(default_quiet_hours_enabled)
        .bind(default_quiet_hours_start)
        .bind(default_quiet_hours_end)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete role defaults.
    pub async fn delete_role_defaults(
        &self,
        organization_id: Uuid,
        role: &str,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            "DELETE FROM role_notification_defaults WHERE organization_id = $1 AND role = $2",
        )
        .bind(organization_id)
        .bind(role)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Apply role defaults to a new user.
    pub async fn apply_role_defaults_to_user(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        role: &str,
    ) -> Result<i64, SqlxError> {
        // Get role defaults
        let defaults = match self.get_role_default(organization_id, role).await? {
            Some(d) => d,
            None => return Ok(0),
        };

        // Parse event preferences from JSONB
        if let serde_json::Value::Object(prefs) = defaults.event_preferences {
            for (event_type, settings) in prefs {
                if let serde_json::Value::Object(channels) = settings {
                    let push = channels.get("push").and_then(|v| v.as_bool());
                    let email = channels.get("email").and_then(|v| v.as_bool());
                    let in_app = channels.get("in_app").and_then(|v| v.as_bool());

                    self.upsert_event_preference(user_id, &event_type, push, email, in_app)
                        .await?;
                }
            }
        }

        // Apply quiet hours if set
        if defaults.default_quiet_hours_enabled {
            self.upsert_schedule(
                user_id,
                Some(true),
                defaults.default_quiet_hours_start,
                defaults.default_quiet_hours_end,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
        }

        Ok(1)
    }

    // ========================================================================
    // Notification Grouping (Epic 29, Story 29.4)
    // ========================================================================

    /// Add a notification to a group (creates group if needed).
    #[allow(clippy::too_many_arguments)]
    pub async fn add_notification_to_group(
        &self,
        user_id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        group_title: &str,
        event_type: &str,
        title: &str,
        body: Option<&str>,
        data: Option<serde_json::Value>,
        actor_id: Option<Uuid>,
        actor_name: Option<&str>,
    ) -> Result<Uuid, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT add_notification_to_group($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(group_title)
        .bind(event_type)
        .bind(title)
        .bind(body)
        .bind(&data)
        .bind(actor_id)
        .bind(actor_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get(0))
    }

    /// Add a notification to a group on behalf of `user_id`, setting that
    /// user's RLS context for the write.
    ///
    /// `notification_groups` / `grouped_notifications` are RLS-protected with a
    /// `user_id = app.current_user_id` policy. A service-role caller (e.g. the
    /// Epic 2B notification pipeline fanning out to many recipients) has no
    /// per-request user context set, so the plain `add_notification_to_group`
    /// would be blocked by the policy's `WITH CHECK` in production. This variant
    /// wraps the call in a transaction and sets `app.current_user_id` to the
    /// recipient (transaction-local via `set_config(..., is_local => true)`),
    /// so the insert is authorised and the setting cannot leak onto a pooled
    /// connection after commit.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_notification_to_group_for_user(
        &self,
        user_id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        group_title: &str,
        event_type: &str,
        title: &str,
        body: Option<&str>,
        data: Option<serde_json::Value>,
        actor_id: Option<Uuid>,
        actor_name: Option<&str>,
    ) -> Result<Uuid, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Transaction-local: scoped to this tx, auto-reset on commit/rollback.
        sqlx::query("SELECT set_config('app.current_user_id', $1, true)")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            r#"
            SELECT add_notification_to_group($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(group_title)
        .bind(event_type)
        .bind(title)
        .bind(body)
        .bind(&data)
        .bind(actor_id)
        .bind(actor_name)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.get(0))
    }

    /// Get grouped notifications for a user.
    pub async fn get_grouped_notifications(
        &self,
        user_id: Uuid,
        limit: i32,
        offset: i32,
        include_read: bool,
    ) -> Result<Vec<crate::models::NotificationGroupWithNotifications>, SqlxError> {
        use crate::models::{
            GroupedNotification, NotificationGroup, NotificationGroupWithNotifications,
        };

        // Get groups
        let groups = sqlx::query_as::<_, NotificationGroup>(
            r#"
            SELECT * FROM notification_groups
            WHERE user_id = $1 AND ($4 OR NOT is_read)
            ORDER BY latest_notification_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .bind(include_read)
        .fetch_all(&self.pool)
        .await?;

        // For each group, get the first 5 notifications
        let mut results = Vec::with_capacity(groups.len());
        for group in groups {
            let notifications = sqlx::query_as::<_, GroupedNotification>(
                r#"
                SELECT * FROM grouped_notifications
                WHERE group_id = $1
                ORDER BY created_at DESC
                LIMIT 5
                "#,
            )
            .bind(group.id)
            .fetch_all(&self.pool)
            .await?;

            results.push(NotificationGroupWithNotifications {
                group,
                notifications,
            });
        }

        Ok(results)
    }

    /// Get total unread notification groups for a user.
    pub async fn get_unread_group_count(&self, user_id: Uuid) -> Result<i64, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM notification_groups
            WHERE user_id = $1 AND is_read = false
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    /// Mark a notification group as read.
    pub async fn mark_group_read(&self, user_id: Uuid, group_id: Uuid) -> Result<bool, SqlxError> {
        let row = sqlx::query("SELECT mark_notification_group_read($1, $2)")
            .bind(user_id)
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get(0))
    }

    /// Mark all notification groups as read.
    pub async fn mark_all_groups_read(&self, user_id: Uuid) -> Result<i32, SqlxError> {
        let row = sqlx::query("SELECT mark_all_notification_groups_read($1)")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get(0))
    }

    /// Get notifications in a group.
    pub async fn get_group_notifications(
        &self,
        group_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<crate::models::GroupedNotification>, SqlxError> {
        sqlx::query_as::<_, crate::models::GroupedNotification>(
            r#"
            SELECT * FROM grouped_notifications
            WHERE group_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(group_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a notification group.
    pub async fn delete_group(&self, user_id: Uuid, group_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query("DELETE FROM notification_groups WHERE id = $1 AND user_id = $2")
            .bind(group_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Notification Digests (Epic 29, Story 29.3)
    // ========================================================================

    /// Create a notification digest.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_digest(
        &self,
        user_id: Uuid,
        digest_type: &str,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
        notification_count: i32,
        category_counts: serde_json::Value,
        summary_html: Option<&str>,
        summary_text: Option<&str>,
    ) -> Result<crate::models::NotificationDigest, SqlxError> {
        sqlx::query_as::<_, crate::models::NotificationDigest>(
            r#"
            INSERT INTO notification_digests (
                user_id, digest_type, period_start, period_end,
                notification_count, category_counts, summary_html, summary_text
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(digest_type)
        .bind(period_start)
        .bind(period_end)
        .bind(notification_count)
        .bind(&category_counts)
        .bind(summary_html)
        .bind(summary_text)
        .fetch_one(&self.pool)
        .await
    }

    /// Get recent digests for a user.
    pub async fn get_user_digests(
        &self,
        user_id: Uuid,
        limit: i32,
    ) -> Result<Vec<crate::models::NotificationDigest>, SqlxError> {
        sqlx::query_as::<_, crate::models::NotificationDigest>(
            r#"
            SELECT * FROM notification_digests
            WHERE user_id = $1
            ORDER BY period_end DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Mark digest as sent via email.
    pub async fn mark_digest_email_sent(&self, digest_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE notification_digests SET email_sent_at = now() WHERE id = $1")
            .bind(digest_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark digest as sent via push.
    pub async fn mark_digest_push_sent(&self, digest_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE notification_digests SET push_sent_at = now() WHERE id = $1")
            .bind(digest_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Find users eligible for a 24h-inactivity email digest (Story 2B.3,
    /// PM #969 gap 1). A user qualifies when they:
    ///   * have opted in via `notification_schedule.digest_enabled = true`,
    ///   * have been inactive since `inactivity_cutoff` (no refresh-token use —
    ///     the closest proxy for "last seen", since `users` has no `last_seen`),
    ///   * are an active, non-deleted account,
    ///   * have at least one unread notification group, and
    ///   * have not had any digest row created since `resend_cutoff` (this
    ///     dedup also prevents re-creating a row for a digest whose email send
    ///     failed — those are retried in place via [`Self::get_unsent_digests`]).
    ///
    /// `category_counts` aggregates unread per `entity_type` (fault, vote, …)
    /// and `notification_count` is the total across all categories.
    pub async fn find_inactive_digest_candidates(
        &self,
        inactivity_cutoff: chrono::DateTime<Utc>,
        resend_cutoff: chrono::DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<InactiveDigestCandidate>, SqlxError> {
        sqlx::query_as::<_, InactiveDigestCandidate>(
            r#"
            WITH last_activity AS (
                SELECT user_id, MAX(last_used_at) AS last_active
                FROM refresh_tokens
                GROUP BY user_id
            ),
            unread AS (
                SELECT user_id,
                       SUM(cat_count)::bigint AS notification_count,
                       jsonb_object_agg(entity_type, cat_count) AS category_counts,
                       MIN(first_at) AS period_start,
                       MAX(last_at) AS period_end
                FROM (
                    SELECT user_id,
                           entity_type,
                           SUM(notification_count)::bigint AS cat_count,
                           MIN(created_at) AS first_at,
                           MAX(latest_notification_at) AS last_at
                    FROM notification_groups
                    WHERE is_read = false
                    GROUP BY user_id, entity_type
                ) per_cat
                GROUP BY user_id
            )
            SELECT u.id AS user_id,
                   u.email,
                   u.locale,
                   un.notification_count,
                   un.category_counts,
                   un.period_start,
                   un.period_end
            FROM users u
            JOIN notification_schedule s ON s.user_id = u.id AND s.digest_enabled = true
            JOIN unread un ON un.user_id = u.id
            JOIN last_activity la ON la.user_id = u.id
            WHERE u.status = 'active'
              AND u.deleted_at IS NULL
              AND la.last_active < $1
              AND NOT EXISTS (
                  SELECT 1 FROM notification_digests d
                  WHERE d.user_id = u.id
                    AND d.created_at > $2
              )
            ORDER BY un.period_end ASC
            LIMIT $3
            "#,
        )
        .bind(inactivity_cutoff)
        .bind(resend_cutoff)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Fetch digest rows whose email has not yet been sent and that were
    /// created since `created_after`, joined with the recipient's address.
    /// Used to retry transient `EmailService` failures without losing the
    /// already-persisted digest row or creating duplicates. Story 2B.3.
    pub async fn get_unsent_digests(
        &self,
        created_after: chrono::DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<PendingDigestEmail>, SqlxError> {
        sqlx::query_as::<_, PendingDigestEmail>(
            r#"
            SELECT d.id AS digest_id,
                   d.user_id,
                   u.email,
                   u.locale,
                   d.notification_count,
                   d.category_counts,
                   d.period_start,
                   d.period_end
            FROM notification_digests d
            JOIN users u ON u.id = d.user_id
            WHERE d.email_sent_at IS NULL
              AND d.created_at > $1
              AND u.status = 'active'
              AND u.deleted_at IS NULL
            ORDER BY d.created_at ASC
            LIMIT $2
            "#,
        )
        .bind(created_after)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Add notification to digest.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_notification_to_digest(
        &self,
        digest_id: Uuid,
        event_type: &str,
        event_category: &str,
        title: &str,
        body: Option<&str>,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
        created_at: chrono::DateTime<Utc>,
    ) -> Result<crate::models::DigestNotification, SqlxError> {
        sqlx::query_as::<_, crate::models::DigestNotification>(
            r#"
            INSERT INTO digest_notifications (
                digest_id, event_type, event_category, title, body,
                entity_type, entity_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(digest_id)
        .bind(event_type)
        .bind(event_category)
        .bind(title)
        .bind(body)
        .bind(entity_type)
        .bind(entity_id)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
    }

    /// Get notifications in a digest.
    pub async fn get_digest_notifications(
        &self,
        digest_id: Uuid,
    ) -> Result<Vec<crate::models::DigestNotification>, SqlxError> {
        sqlx::query_as::<_, crate::models::DigestNotification>(
            r#"
            SELECT * FROM digest_notifications
            WHERE digest_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(digest_id)
        .fetch_all(&self.pool)
        .await
    }
}

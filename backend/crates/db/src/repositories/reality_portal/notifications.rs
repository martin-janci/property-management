//! Portal user notification settings and alert inbox housekeeping.

use super::RealityPortalRepository;
use crate::models::reality_portal::PortalNotificationPreferences;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RealityPortalRepository {
    /// Read a user's notification preferences, falling back to the defaults
    /// when the user never saved any.
    pub async fn get_notification_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<PortalNotificationPreferences, SqlxError> {
        let row = sqlx::query_as::<_, PortalNotificationPreferences>(
            r#"
            SELECT new_listings, price_drops, inquiry_responses, listing_updates, marketing
            FROM portal_notification_preferences
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.unwrap_or_default())
    }

    /// Replace a user's notification preferences (upsert).
    pub async fn upsert_notification_preferences(
        &self,
        user_id: Uuid,
        prefs: PortalNotificationPreferences,
    ) -> Result<PortalNotificationPreferences, SqlxError> {
        sqlx::query_as::<_, PortalNotificationPreferences>(
            r#"
            INSERT INTO portal_notification_preferences
                (user_id, new_listings, price_drops, inquiry_responses, listing_updates, marketing)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id) DO UPDATE SET
                new_listings      = EXCLUDED.new_listings,
                price_drops       = EXCLUDED.price_drops,
                inquiry_responses = EXCLUDED.inquiry_responses,
                listing_updates   = EXCLUDED.listing_updates,
                marketing         = EXCLUDED.marketing,
                updated_at        = NOW()
            RETURNING new_listings, price_drops, inquiry_responses, listing_updates, marketing
            "#,
        )
        .bind(user_id)
        .bind(prefs.new_listings)
        .bind(prefs.price_drops)
        .bind(prefs.inquiry_responses)
        .bind(prefs.listing_updates)
        .bind(prefs.marketing)
        .fetch_one(&self.pool)
        .await
    }

    /// Dismiss (delete) one of the user's favorite alerts. Returns `false`
    /// when the alert does not exist or belongs to someone else.
    pub async fn delete_favorite_alert(&self, id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let res = sqlx::query("DELETE FROM favorite_alert_queue WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}

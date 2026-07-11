//! iCal feed CRUD.

use super::RentalRepository;
use super::ICAL_FEED_COLUMNS;
use crate::models::rental::{CreateICalFeed, ICalFeed, UpdateICalFeed};
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // iCal Feeds
    // ========================================================================

    /// Create iCal feed.
    pub async fn create_ical_feed(
        &self,
        org_id: Uuid,
        data: CreateICalFeed,
    ) -> Result<ICalFeed, SqlxError> {
        let token = Uuid::new_v4().to_string().replace("-", "");

        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_ical_feeds (
                organization_id, unit_id, feed_name, feed_token,
                import_url, import_platform
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING {ICAL_FEED_COLUMNS}
            "#
        )))
        .bind(org_id)
        .bind(data.unit_id)
        .bind(&data.feed_name)
        .bind(&token)
        .bind(&data.import_url)
        .bind(&data.import_platform)
        .fetch_one(&self.pool)
        .await?;

        Ok(feed)
    }

    /// Find iCal feed by token.
    pub async fn find_ical_feed_by_token(
        &self,
        token: &str,
    ) -> Result<Option<ICalFeed>, SqlxError> {
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            "SELECT {ICAL_FEED_COLUMNS} FROM rental_ical_feeds WHERE feed_token = $1 AND is_active = true"
        )))
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(feed)
    }

    /// Get iCal feeds for unit.
    pub async fn get_ical_feeds_for_unit(&self, unit_id: Uuid) -> Result<Vec<ICalFeed>, SqlxError> {
        let feeds = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            "SELECT {ICAL_FEED_COLUMNS} FROM rental_ical_feeds WHERE unit_id = $1 ORDER BY feed_name"
        )))
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(feeds)
    }

    /// Get iCal feeds for a unit scoped to an organization.
    ///
    /// SECURITY (#804): adds `AND organization_id = $2` so a caller cannot
    /// enumerate another org's feeds by guessing a unit UUID.
    pub async fn get_ical_feeds_for_unit_in_org(
        &self,
        org_id: Uuid,
        unit_id: Uuid,
    ) -> Result<Vec<ICalFeed>, SqlxError> {
        let feeds = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            "SELECT {ICAL_FEED_COLUMNS} FROM rental_ical_feeds WHERE unit_id = $1 AND organization_id = $2 ORDER BY feed_name"
        )))
        .bind(unit_id)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(feeds)
    }

    /// Update iCal feed scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $5` guard prevents a tenant
    /// from mutating another org's feed. Returns `None` when no row matched.
    pub async fn update_ical_feed_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        data: UpdateICalFeed,
    ) -> Result<Option<ICalFeed>, SqlxError> {
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_ical_feeds SET
                feed_name = COALESCE($2, feed_name),
                import_url = COALESCE($3, import_url),
                is_active = COALESCE($4, is_active),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $5
            RETURNING {ICAL_FEED_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(&data.feed_name)
        .bind(&data.import_url)
        .bind(data.is_active)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(feed)
    }

    /// Delete iCal feed scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $2` guard prevents a tenant
    /// from deleting another org's feed.
    pub async fn delete_ical_feed_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result =
            sqlx::query(r#"DELETE FROM rental_ical_feeds WHERE id = $1 AND organization_id = $2"#)
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update iCal feed.
    pub async fn update_ical_feed(
        &self,
        id: Uuid,
        data: UpdateICalFeed,
    ) -> Result<ICalFeed, SqlxError> {
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_ical_feeds SET
                feed_name = COALESCE($2, feed_name),
                import_url = COALESCE($3, import_url),
                is_active = COALESCE($4, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {ICAL_FEED_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(&data.feed_name)
        .bind(&data.import_url)
        .bind(data.is_active)
        .fetch_one(&self.pool)
        .await?;

        Ok(feed)
    }

    /// Delete iCal feed.
    pub async fn delete_ical_feed(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM rental_ical_feeds WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

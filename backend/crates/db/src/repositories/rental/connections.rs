//! Rental platform connection CRUD (Story 18.1).

use super::RentalRepository;
use crate::models::rental::{
    ConnectionStatus, CreatePlatformConnection, PlatformConnectionSummary,
    RentalPlatformConnection, UpdatePlatformConnection,
};
use chrono::Utc;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // Platform Connections (Story 18.1)
    // ========================================================================

    /// Create platform connection.
    pub async fn create_connection(
        &self,
        org_id: Uuid,
        data: CreatePlatformConnection,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(
            r#"
            INSERT INTO rental_platform_connections (
                organization_id, unit_id, platform, external_property_id,
                sync_calendar, sync_interval_minutes, block_other_platforms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, organization_id, unit_id, platform::text AS platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, external_listing_url,
                is_active, last_sync_at, sync_error,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(&data.platform)
        .bind(&data.external_property_id)
        .bind(data.sync_calendar)
        .bind(data.sync_interval_minutes)
        .bind(data.block_other_platforms)
        .fetch_one(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Find connection by ID scoped to an organization.
    ///
    /// SECURITY (#887 / #804 / PAP-141): this is the ONLY by-id connection
    /// lookup — the unkeyed `find_connection_by_id` was removed with the legacy
    /// OAuth callbacks. The `AND organization_id = $2` guard means a caller from
    /// org B cannot read org A's connection (and its OAuth tokens). Returns
    /// `None` when the connection does not exist OR belongs to a different
    /// organization (the handler maps `None` → 404).
    pub async fn find_connection_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<RentalPlatformConnection>, SqlxError> {
        // `platform` is the `rental_platform` enum; the model decodes it as
        // `String`, so it must be cast with `platform::text` (a bare `SELECT *`
        // panics with "mismatched types … not compatible with SQL type
        // rental_platform"). All other columns map 1:1 to the model fields.
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(
            r#"
            SELECT
                id, organization_id, unit_id, platform::text AS platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, external_listing_url,
                is_active, last_sync_at, sync_error,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            FROM rental_platform_connections
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Find connection by unit and platform.
    pub async fn find_connection_by_unit_platform(
        &self,
        unit_id: Uuid,
        platform: &str,
    ) -> Result<Option<RentalPlatformConnection>, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(
            r#"
            SELECT
                id, organization_id, unit_id, platform::text AS platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, external_listing_url,
                is_active, last_sync_at, sync_error,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            FROM rental_platform_connections
            WHERE unit_id = $1 AND platform = $2
            "#,
        )
        .bind(unit_id)
        .bind(platform)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Update connection.
    pub async fn update_connection(
        &self,
        id: Uuid,
        data: UpdatePlatformConnection,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(
            r#"
            UPDATE rental_platform_connections SET
                external_property_id = COALESCE($2, external_property_id),
                external_listing_url = COALESCE($3, external_listing_url),
                is_active = COALESCE($4, is_active),
                sync_calendar = COALESCE($5, sync_calendar),
                sync_interval_minutes = COALESCE($6, sync_interval_minutes),
                block_other_platforms = COALESCE($7, block_other_platforms),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, organization_id, unit_id, platform::text AS platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, external_listing_url,
                is_active, last_sync_at, sync_error,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.external_property_id)
        .bind(&data.external_listing_url)
        .bind(data.is_active)
        .bind(data.sync_calendar)
        .bind(data.sync_interval_minutes)
        .bind(data.block_other_platforms)
        .fetch_one(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Update connection scoped to an organization.
    ///
    /// SECURITY (#887 / #804): the `AND organization_id = $8` guard prevents a
    /// tenant from mutating another org's connection. Returns `None` when no
    /// row matched (missing or cross-org) so the handler can return 404.
    pub async fn update_connection_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        data: UpdatePlatformConnection,
    ) -> Result<Option<RentalPlatformConnection>, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(
            r#"
            UPDATE rental_platform_connections SET
                external_property_id = COALESCE($2, external_property_id),
                external_listing_url = COALESCE($3, external_listing_url),
                is_active = COALESCE($4, is_active),
                sync_calendar = COALESCE($5, sync_calendar),
                sync_interval_minutes = COALESCE($6, sync_interval_minutes),
                block_other_platforms = COALESCE($7, block_other_platforms),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $8
            RETURNING
                id, organization_id, unit_id, platform::text AS platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, external_listing_url,
                is_active, last_sync_at, sync_error,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.external_property_id)
        .bind(&data.external_listing_url)
        .bind(data.is_active)
        .bind(data.sync_calendar)
        .bind(data.sync_interval_minutes)
        .bind(data.block_other_platforms)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Update last sync time.
    pub async fn update_sync_status(&self, id: Uuid, error: Option<&str>) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE rental_platform_connections SET
                last_sync_at = NOW(),
                sync_error = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete connection.
    pub async fn delete_connection(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM rental_platform_connections WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete connection scoped to an organization.
    ///
    /// SECURITY (#887 / #804): the `AND organization_id = $2` guard prevents a
    /// tenant from deleting another org's connection.
    pub async fn delete_connection_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"DELETE FROM rental_platform_connections WHERE id = $1 AND organization_id = $2"#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get connection statuses for a unit scoped to an organization.
    ///
    /// SECURITY (#887 / #804): adds `AND organization_id = $2` so a caller
    /// cannot enumerate another org's unit connections by guessing a unit UUID.
    pub async fn get_connections_for_unit_in_org(
        &self,
        org_id: Uuid,
        unit_id: Uuid,
    ) -> Result<Vec<ConnectionStatus>, SqlxError> {
        let connections = sqlx::query_as::<_, (Uuid, String, bool, bool, Option<chrono::DateTime<Utc>>, Option<String>, Option<String>)>(
            r#"
            SELECT id, platform::text, access_token IS NOT NULL, is_active, last_sync_at, sync_error, external_listing_url
            FROM rental_platform_connections
            WHERE unit_id = $1 AND organization_id = $2
            ORDER BY platform
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        let statuses = connections
            .into_iter()
            .map(
                |(
                    id,
                    platform,
                    is_connected,
                    is_active,
                    last_sync_at,
                    sync_error,
                    external_listing_url,
                )| {
                    ConnectionStatus {
                        id,
                        platform,
                        is_connected,
                        is_active,
                        last_sync_at,
                        sync_error,
                        external_listing_url,
                    }
                },
            )
            .collect();

        Ok(statuses)
    }

    /// Get connections for organization.
    pub async fn get_connections_for_org(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PlatformConnectionSummary>, SqlxError> {
        let connections = sqlx::query_as::<_, PlatformConnectionSummary>(
            r#"
            SELECT
                c.id, c.unit_id, u.designation as unit_name,
                c.platform::text, c.is_active, c.last_sync_at, c.sync_error
            FROM rental_platform_connections c
            JOIN units u ON u.id = c.unit_id
            WHERE c.organization_id = $1
            ORDER BY u.designation, c.platform
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(connections)
    }

    /// Get connections for unit.
    pub async fn get_connections_for_unit(
        &self,
        unit_id: Uuid,
    ) -> Result<Vec<ConnectionStatus>, SqlxError> {
        let connections = sqlx::query_as::<_, (Uuid, String, bool, bool, Option<chrono::DateTime<Utc>>, Option<String>, Option<String>)>(
            r#"
            SELECT id, platform::text, access_token IS NOT NULL, is_active, last_sync_at, sync_error, external_listing_url
            FROM rental_platform_connections
            WHERE unit_id = $1
            ORDER BY platform
            "#,
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;

        let statuses = connections
            .into_iter()
            .map(
                |(
                    id,
                    platform,
                    is_connected,
                    is_active,
                    last_sync_at,
                    sync_error,
                    external_listing_url,
                )| {
                    ConnectionStatus {
                        id,
                        platform,
                        is_connected,
                        is_active,
                        last_sync_at,
                        sync_error,
                        external_listing_url,
                    }
                },
            )
            .collect();

        Ok(statuses)
    }

    /// Get connections needing sync.
    pub async fn get_connections_needing_sync(
        &self,
    ) -> Result<Vec<RentalPlatformConnection>, SqlxError> {
        let connections = sqlx::query_as::<_, RentalPlatformConnection>(
            r#"
            SELECT
                id, organization_id, unit_id, platform::text AS platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, external_listing_url,
                is_active, last_sync_at, sync_error,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            FROM rental_platform_connections
            WHERE is_active = true
                AND sync_calendar = true
                AND access_token IS NOT NULL
                AND (
                    last_sync_at IS NULL
                    OR last_sync_at < NOW() - INTERVAL '1 minute' * sync_interval_minutes
                )
            ORDER BY last_sync_at NULLS FIRST
            LIMIT 100
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(connections)
    }
}

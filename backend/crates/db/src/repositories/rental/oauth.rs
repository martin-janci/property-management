//! Airbnb / Booking.com OAuth token management (Story 96.2 / 98.5).

use super::RentalRepository;
use super::PLATFORM_CONNECTION_COLUMNS;
use crate::models::rental::RentalPlatformConnection;
use chrono::{Duration, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // OAuth Token Management (Story 96.2)
    // ========================================================================

    /// Find Airbnb connection by organization.
    pub async fn find_airbnb_connection_by_org(
        &self,
        org_id: Uuid,
    ) -> Result<Option<RentalPlatformConnection>, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {PLATFORM_CONNECTION_COLUMNS} FROM rental_platform_connections
            WHERE organization_id = $1 AND platform = 'airbnb'
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )))
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Find Airbnb connection by listing ID (external_property_id).
    ///
    /// # Why `sqlx::query_as` instead of `query_as!` macro
    ///
    /// The compile-time `query_as!` macro requires a live database connection
    /// (or a pre-generated `.sqlx/` cache via `cargo sqlx prepare`) at build time.
    /// This function was added in gap-83-1 where the CI/CD environment uses
    /// `SQLX_OFFLINE=true` and the offline query cache has not yet been regenerated
    /// for the new `rental_platform_connections` table columns added in migration
    /// `00051_create_short_term_rentals.sql`.
    ///
    /// TODO(gap-83-1): Convert to `query_as!` after running `cargo sqlx prepare`
    /// against a fully-migrated database and committing the updated `.sqlx/` cache.
    pub async fn find_airbnb_connection_by_listing_id(
        &self,
        listing_id: &str,
    ) -> Result<Option<RentalPlatformConnection>, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {PLATFORM_CONNECTION_COLUMNS} FROM rental_platform_connections
            WHERE platform = 'airbnb' AND external_property_id = $1 AND is_active = true
            ORDER BY updated_at DESC
            LIMIT 1
            "#
        )))
        .bind(listing_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conn)
    }

    /// List all stored Airbnb connections (linked listings) for an organisation.
    ///
    /// Coverage 83-1: this is the DB-backed read of the org's *stored* Airbnb
    /// connection rows, as opposed to the live-proxied `/airbnb/listings` route
    /// which calls the Airbnb Partner API through the stored token. It returns
    /// every `platform = 'airbnb'` row for the org (active or not) so the
    /// management UI can render the linked-listing list without any external
    /// network round-trip — including org-level (nil `unit_id`) connections that
    /// the `units`-joined `get_connections_for_org` summary would drop.
    ///
    /// Scoped to `organization_id = $1`; rows are returned newest-first.
    pub async fn list_airbnb_connections(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<RentalPlatformConnection>, SqlxError> {
        let connections =
            sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
                r#"
            SELECT {PLATFORM_CONNECTION_COLUMNS} FROM rental_platform_connections
            WHERE organization_id = $1 AND platform = 'airbnb'
            ORDER BY created_at DESC
            "#
            )))
            .bind(org_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(connections)
    }

    /// Record an inbound Airbnb webhook delivery in the dedup ledger.
    ///
    /// Airbnb guarantees at-least-once delivery; this inserts the delivery's
    /// dedup key into the global `airbnb_webhook_events` ledger and reports
    /// whether the row was newly recorded. `ON CONFLICT DO NOTHING` makes a
    /// re-delivery a no-op, so `Ok(false)` means "already seen — suppress".
    ///
    /// PAP-170 (PAP-150): `airbnb_webhook_events` is a global, tenant-less dedup
    /// table (no `organization_id`), so this lives in the repository layer
    /// instead of a raw `state.db` pool access inside the webhook handler.
    pub async fn record_airbnb_webhook_event(
        &self,
        event_id: &str,
        event_type: &str,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "INSERT INTO airbnb_webhook_events (event_id, event_type) \
             VALUES ($1, $2) ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(event_id)
        .bind(event_type)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get Airbnb status for organization (aggregated from all connections).
    pub async fn get_airbnb_status(
        &self,
        org_id: Uuid,
    ) -> Result<(i64, i64, Option<chrono::DateTime<Utc>>, Option<String>), SqlxError> {
        let result = sqlx::query_as::<_, (i64, i64, Option<chrono::DateTime<Utc>>, Option<String>)>(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE is_active AND access_token IS NOT NULL) as connected_count,
                COUNT(DISTINCT external_property_id) FILTER (WHERE external_property_id IS NOT NULL) as listings_count,
                MAX(last_sync_at) as last_sync,
                (SELECT sync_error FROM rental_platform_connections
                 WHERE organization_id = $1 AND platform = 'airbnb' AND sync_error IS NOT NULL
                 ORDER BY updated_at DESC LIMIT 1) as last_error
            FROM rental_platform_connections
            WHERE organization_id = $1 AND platform = 'airbnb'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Create or update Airbnb connection with OAuth tokens.
    pub async fn upsert_airbnb_connection(
        &self,
        org_id: Uuid,
        unit_id: Option<Uuid>,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<Utc>>,
        external_account_id: Option<&str>,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        let effective_unit_id = unit_id.unwrap_or_else(Uuid::nil);

        // Org-level (nil unit_id) connections use the partial unique index
        // (organization_id, platform) WHERE unit_id = nil so that two orgs can
        // both hold an org-level Airbnb row without colliding on the per-unit
        // (unit_id, platform) constraint — and so one org's DO UPDATE can never
        // silently rebind another org's row. (BIT-85 cross-tenant hazard fix.)
        if effective_unit_id == Uuid::nil() {
            let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
                r#"
                INSERT INTO rental_platform_connections (
                    organization_id, unit_id, platform,
                    access_token, refresh_token, token_expires_at,
                    encrypted_token, encrypted_refresh_token,
                    external_property_id, is_active
                )
                VALUES ($1, $2, 'airbnb', $3, $4, $5, $3, $4, $6, true)
                ON CONFLICT (organization_id, platform) WHERE unit_id = '00000000-0000-0000-0000-000000000000'
                DO UPDATE SET
                    organization_id          = $1,
                    access_token             = $3,
                    refresh_token            = COALESCE($4, rental_platform_connections.refresh_token),
                    encrypted_token          = $3,
                    encrypted_refresh_token  = COALESCE($4, rental_platform_connections.encrypted_refresh_token),
                    token_expires_at         = $5,
                    external_property_id     = COALESCE($6, rental_platform_connections.external_property_id),
                    is_active                = true,
                    sync_error               = NULL,
                    updated_at               = NOW()
                RETURNING {PLATFORM_CONNECTION_COLUMNS}
                "#
            )))
            .bind(org_id)
            .bind(effective_unit_id)
            .bind(access_token)
            .bind(refresh_token)
            .bind(expires_at)
            .bind(external_account_id)
            .fetch_one(&self.pool)
            .await?;
            return Ok(conn);
        }

        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_platform_connections (
                organization_id, unit_id, platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, is_active
            )
            VALUES ($1, $2, 'airbnb', $3, $4, $5, $3, $4, $6, true)
            ON CONFLICT (unit_id, platform) WHERE unit_id <> '00000000-0000-0000-0000-000000000000' DO UPDATE SET
                organization_id          = $1,
                access_token             = $3,
                refresh_token            = COALESCE($4, rental_platform_connections.refresh_token),
                encrypted_token          = $3,
                encrypted_refresh_token  = COALESCE($4, rental_platform_connections.encrypted_refresh_token),
                token_expires_at         = $5,
                external_property_id     = COALESCE($6, rental_platform_connections.external_property_id),
                is_active                = true,
                sync_error               = NULL,
                updated_at               = NOW()
            RETURNING {PLATFORM_CONNECTION_COLUMNS}
            "#
        )))
        .bind(org_id)
        .bind(effective_unit_id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .bind(external_account_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Create or update a Booking.com OAuth connection (Coverage 83-2).
    ///
    /// Mirrors [`Self::upsert_airbnb_connection`] but with `platform =
    /// 'booking'`.  Tokens are expected to be already encrypted by the caller.
    pub async fn upsert_booking_oauth_connection(
        &self,
        org_id: Uuid,
        unit_id: Option<Uuid>,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<Utc>>,
        external_property_id: Option<&str>,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        let effective_unit_id = unit_id.unwrap_or_else(Uuid::nil);

        // Same org-scoped conflict target as upsert_airbnb_connection for the
        // nil-unit_id case. (BIT-85 cross-tenant hazard fix.)
        if effective_unit_id == Uuid::nil() {
            let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
                r#"
                INSERT INTO rental_platform_connections (
                    organization_id, unit_id, platform,
                    access_token, refresh_token, token_expires_at,
                    encrypted_token, encrypted_refresh_token,
                    external_property_id, is_active
                )
                VALUES ($1, $2, 'booking', $3, $4, $5, $3, $4, $6, true)
                ON CONFLICT (organization_id, platform) WHERE unit_id = '00000000-0000-0000-0000-000000000000'
                DO UPDATE SET
                    organization_id          = $1,
                    access_token             = $3,
                    refresh_token            = COALESCE($4, rental_platform_connections.refresh_token),
                    encrypted_token          = $3,
                    encrypted_refresh_token  = COALESCE($4, rental_platform_connections.encrypted_refresh_token),
                    token_expires_at         = $5,
                    external_property_id     = COALESCE($6, rental_platform_connections.external_property_id),
                    is_active                = true,
                    sync_error               = NULL,
                    updated_at               = NOW()
                RETURNING {PLATFORM_CONNECTION_COLUMNS}
                "#
            )))
            .bind(org_id)
            .bind(effective_unit_id)
            .bind(access_token)
            .bind(refresh_token)
            .bind(expires_at)
            .bind(external_property_id)
            .fetch_one(&self.pool)
            .await?;
            return Ok(conn);
        }

        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_platform_connections (
                organization_id, unit_id, platform,
                access_token, refresh_token, token_expires_at,
                encrypted_token, encrypted_refresh_token,
                external_property_id, is_active
            )
            VALUES ($1, $2, 'booking', $3, $4, $5, $3, $4, $6, true)
            ON CONFLICT (unit_id, platform) WHERE unit_id <> '00000000-0000-0000-0000-000000000000' DO UPDATE SET
                organization_id          = $1,
                access_token             = $3,
                refresh_token            = COALESCE($4, rental_platform_connections.refresh_token),
                encrypted_token          = $3,
                encrypted_refresh_token  = COALESCE($4, rental_platform_connections.encrypted_refresh_token),
                token_expires_at         = $5,
                external_property_id     = COALESCE($6, rental_platform_connections.external_property_id),
                is_active                = true,
                sync_error               = NULL,
                updated_at               = NOW()
            RETURNING {PLATFORM_CONNECTION_COLUMNS}
            "#
        )))
        .bind(org_id)
        .bind(effective_unit_id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .bind(external_property_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Store listing ID mapping for Airbnb connection.
    pub async fn update_airbnb_listing_mapping(
        &self,
        connection_id: Uuid,
        external_property_id: &str,
        external_listing_url: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE rental_platform_connections SET
                external_property_id = $2,
                external_listing_url = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(connection_id)
        .bind(external_property_id)
        .bind(external_listing_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Revoke Airbnb connection (clear tokens).
    pub async fn revoke_airbnb_connection(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE rental_platform_connections SET
                access_token = NULL,
                refresh_token = NULL,
                encrypted_token = NULL,
                encrypted_refresh_token = NULL,
                token_expires_at = NULL,
                is_active = false,
                sync_error = 'User revoked access',
                updated_at = NOW()
            WHERE organization_id = $1 AND platform = 'airbnb'
            "#,
        )
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Rotate Airbnb OAuth tokens for a connection after a successful refresh.
    ///
    /// Gap 83-1: Writes to both the canonical `encrypted_token` /
    /// `encrypted_refresh_token` columns (added in migration 00175) AND the
    /// legacy `access_token` / `refresh_token` columns so that code that
    /// hasn't been updated yet keeps working.
    pub async fn update_airbnb_tokens(
        &self,
        connection_id: Uuid,
        encrypted_access: &str,
        encrypted_refresh: Option<&str>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_platform_connections SET
                access_token             = $2,
                refresh_token            = COALESCE($3, refresh_token),
                encrypted_token          = $2,
                encrypted_refresh_token  = COALESCE($3, encrypted_refresh_token),
                token_expires_at         = $4,
                sync_error               = NULL,
                updated_at               = NOW()
            WHERE id = $1
              AND platform = 'airbnb'
            RETURNING {PLATFORM_CONNECTION_COLUMNS}
            "#
        )))
        .bind(connection_id)
        .bind(encrypted_access)
        .bind(encrypted_refresh)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(conn)
    }

    /// Mark an Airbnb connection as broken due to a permanent auth failure.
    ///
    /// Records the error and disables the connection so operators can see the
    /// problem in the status dashboard.
    pub async fn mark_airbnb_token_error(
        &self,
        connection_id: Uuid,
        error: &str,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE rental_platform_connections SET
                sync_error = $2,
                updated_at = NOW()
            WHERE id = $1 AND platform = 'airbnb'
            "#,
        )
        .bind(connection_id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get Airbnb connections needing token refresh.
    ///
    /// Gap 83-1: Checks both `encrypted_refresh_token` (canonical) and the
    /// legacy `refresh_token` column so that connections migrated by 00175
    /// are also picked up.
    pub async fn get_airbnb_connections_needing_refresh(
        &self,
        buffer_secs: i64,
    ) -> Result<Vec<RentalPlatformConnection>, SqlxError> {
        let threshold = Utc::now() + Duration::seconds(buffer_secs);

        let connections =
            sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
                r#"
            SELECT {PLATFORM_CONNECTION_COLUMNS} FROM rental_platform_connections
            WHERE platform = 'airbnb'
              AND is_active = true
              AND (encrypted_refresh_token IS NOT NULL OR refresh_token IS NOT NULL)
              AND token_expires_at IS NOT NULL
              AND token_expires_at <= $1
            ORDER BY token_expires_at ASC
            LIMIT 100
            "#
            )))
            .bind(threshold)
            .fetch_all(&self.pool)
            .await?;

        Ok(connections)
    }

    /// Count Airbnb reservations for organization.
    pub async fn count_airbnb_reservations(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rental_bookings
            WHERE organization_id = $1 AND platform = 'airbnb'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    // ========================================================================
    // Booking.com Connection Methods (Story 98.5)
    // ========================================================================

    /// Find Booking.com connection by organization.
    pub async fn find_booking_connection_by_org(
        &self,
        org_id: Uuid,
    ) -> Result<Option<RentalPlatformConnection>, SqlxError> {
        let connection =
            sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
                r#"
            SELECT {PLATFORM_CONNECTION_COLUMNS} FROM rental_platform_connections
            WHERE organization_id = $1 AND platform = 'booking'
            ORDER BY created_at DESC
            LIMIT 1
            "#
            )))
            .bind(org_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(connection)
    }

    /// Create or update Booking.com connection.
    /// For Booking.com, we store credentials in access_token (username) and refresh_token (password)
    pub async fn create_or_update_booking_connection(
        &self,
        org_id: Uuid,
        hotel_id: &str,
        username: &str,
        password: &str,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        let id = Uuid::new_v4();
        let unit_id = Uuid::nil(); // Booking connections are org-level

        let connection =
            sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
                r#"
            INSERT INTO rental_platform_connections (
                id, unit_id, organization_id, platform, external_property_id,
                access_token, refresh_token, is_active,
                sync_calendar, sync_interval_minutes, block_other_platforms,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, 'booking', $4, $5, $6, true, false, 60, false, NOW(), NOW())
            ON CONFLICT (organization_id, platform) WHERE unit_id = '00000000-0000-0000-0000-000000000000'
            DO UPDATE SET
                external_property_id = EXCLUDED.external_property_id,
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                is_active = true,
                sync_error = NULL,
                updated_at = NOW()
            RETURNING {PLATFORM_CONNECTION_COLUMNS}
            "#
            )))
            .bind(id)
            .bind(unit_id)
            .bind(org_id)
            .bind(hotel_id)
            .bind(username)
            .bind(password)
            .fetch_one(&self.pool)
            .await?;

        Ok(connection)
    }

    /// Revoke Booking.com connection (clear credentials).
    pub async fn revoke_booking_connection(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE rental_platform_connections SET
                access_token = NULL,
                refresh_token = NULL,
                is_active = false,
                sync_error = 'User revoked access',
                updated_at = NOW()
            WHERE organization_id = $1 AND platform = 'booking'
            "#,
        )
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Update connection last sync timestamp.
    pub async fn update_connection_last_sync(
        &self,
        connection_id: Uuid,
        sync_time: chrono::DateTime<Utc>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE rental_platform_connections SET
                last_sync_at = $2,
                sync_error = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(connection_id)
        .bind(sync_time)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

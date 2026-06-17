//! Rental repository (Epic 18: Short-Term Rental Integration).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::models::rental::{
    block_reason, booking_status, guest_status, report_status, BookingListQuery, BookingSummary,
    BookingWithGuests, CalendarBlock, CalendarEvent, CheckInReminder, ConnectionStatus,
    CreateBooking, CreateCalendarBlock, CreateGuest, CreateICalFeed, CreatePlatformConnection,
    GenerateReport, ICalFeed, NationalityStats, PlatformConnectionSummary, PlatformSyncStatus,
    RentalBooking, RentalGuest, RentalGuestReport, RentalPlatformConnection, RentalStatistics,
    ReportPreview, ReportSummary, UpdateBooking, UpdateBookingStatus, UpdateGuest, UpdateICalFeed,
    UpdatePlatformConnection,
};
use crate::DbPool;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Static mapping of ISO 3166-1 alpha-2 country codes to country names.
/// Includes EU countries and common destinations for short-term rentals.
static COUNTRY_NAMES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // Central Europe
    m.insert("SK", "Slovakia");
    m.insert("CZ", "Czech Republic");
    m.insert("AT", "Austria");
    m.insert("DE", "Germany");
    m.insert("HU", "Hungary");
    m.insert("PL", "Poland");
    m.insert("CH", "Switzerland");
    m.insert("SI", "Slovenia");
    // Eastern Europe
    m.insert("UA", "Ukraine");
    m.insert("RO", "Romania");
    m.insert("BG", "Bulgaria");
    m.insert("MD", "Moldova");
    m.insert("BY", "Belarus");
    m.insert("RU", "Russia");
    // Western Europe
    m.insert("GB", "United Kingdom");
    m.insert("FR", "France");
    m.insert("NL", "Netherlands");
    m.insert("BE", "Belgium");
    m.insert("LU", "Luxembourg");
    m.insert("IE", "Ireland");
    // Southern Europe
    m.insert("ES", "Spain");
    m.insert("IT", "Italy");
    m.insert("PT", "Portugal");
    m.insert("GR", "Greece");
    m.insert("HR", "Croatia");
    m.insert("RS", "Serbia");
    m.insert("ME", "Montenegro");
    m.insert("MK", "North Macedonia");
    m.insert("AL", "Albania");
    m.insert("BA", "Bosnia and Herzegovina");
    m.insert("XK", "Kosovo");
    m.insert("MT", "Malta");
    m.insert("CY", "Cyprus");
    // Northern Europe
    m.insert("SE", "Sweden");
    m.insert("NO", "Norway");
    m.insert("FI", "Finland");
    m.insert("DK", "Denmark");
    m.insert("IS", "Iceland");
    m.insert("EE", "Estonia");
    m.insert("LV", "Latvia");
    m.insert("LT", "Lithuania");
    // Americas
    m.insert("US", "United States");
    m.insert("CA", "Canada");
    m.insert("MX", "Mexico");
    m.insert("BR", "Brazil");
    m.insert("AR", "Argentina");
    // Asia & Middle East
    m.insert("CN", "China");
    m.insert("JP", "Japan");
    m.insert("KR", "South Korea");
    m.insert("IN", "India");
    m.insert("TR", "Turkey");
    m.insert("IL", "Israel");
    m.insert("AE", "United Arab Emirates");
    // Oceania
    m.insert("AU", "Australia");
    m.insert("NZ", "New Zealand");
    // Africa
    m.insert("ZA", "South Africa");
    m.insert("EG", "Egypt");
    m.insert("MA", "Morocco");
    // Special
    m.insert("UNK", "Unknown");
    m
});

/// Get country name from ISO 3166-1 alpha-2 code.
/// Returns the country name if found, otherwise returns the code itself.
fn get_country_name(code: &str) -> String {
    COUNTRY_NAMES
        .get(code.to_uppercase().as_str())
        .map(|name| (*name).to_string())
        .unwrap_or_else(|| code.to_string())
}

/// Repository for rental operations.
#[derive(Clone)]
pub struct RentalRepository {
    pool: DbPool,
}

impl RentalRepository {
    /// Create a new RentalRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Explicit column projection for rental_bookings with enum-to-text casts.
    /// `platform` and `status` are PG enums; bare SELECT * fails ColumnDecode.
    const BOOKING_COLUMNS: &'static str = r#"
        id, organization_id, unit_id, connection_id,
        platform::text AS platform, external_booking_id, external_booking_url,
        guest_name, guest_email, guest_phone, guest_count,
        check_in, check_out, check_in_time, check_out_time,
        total_amount, currency, platform_fee, cleaning_fee,
        status::text AS status, cancelled_at, cancellation_reason,
        guest_notes, internal_notes, synced_at, raw_data,
        created_at, updated_at
    "#;

    /// PAP-158: explicit projection for rental_platform_connections. `platform`
    /// is the `rental_platform` enum but `RentalPlatformConnection.platform` is
    /// `String`, so a bare `SELECT */RETURNING *` panics with ColumnDecode
    /// ("not compatible with SQL type rental_platform"). Cast the enum to text.
    const CONNECTION_COLUMNS: &'static str = r#"
        id, organization_id, unit_id, platform::text AS platform,
        access_token, refresh_token, token_expires_at,
        encrypted_token, encrypted_refresh_token,
        external_property_id, external_listing_url,
        is_active, last_sync_at, sync_error,
        sync_calendar, sync_interval_minutes, block_other_platforms,
        created_at, updated_at
    "#;

    /// PAP-158: explicit projection for rental_guests. `status` is the
    /// `guest_registration_status` enum but `RentalGuest.status` is `String`,
    /// so cast it to text (bare `SELECT */RETURNING *` fails ColumnDecode).
    const GUEST_COLUMNS: &'static str = r#"
        id, organization_id, booking_id,
        first_name, last_name, date_of_birth, nationality,
        id_type, id_number, id_issuing_country, id_expiry_date, id_document_url,
        email, phone,
        address_street, address_city, address_postal_code, address_country,
        status::text AS status, registered_at, reported_at, report_reference,
        is_primary, created_at, updated_at
    "#;

    /// PAP-158: explicit projection for rental_calendar_blocks. `source_platform`
    /// is the nullable `rental_platform` enum but `CalendarBlock.source_platform`
    /// is `Option<String>`, so cast it to text.
    const CALENDAR_BLOCK_COLUMNS: &'static str = r#"
        id, organization_id, unit_id, block_start, block_end,
        reason, booking_id, source_platform::text AS source_platform,
        synced_at, notes, created_at
    "#;

    /// PAP-158: explicit projection for rental_ical_feeds. `import_platform` is
    /// the nullable `rental_platform` enum but `ICalFeed.import_platform` is
    /// `Option<String>`, so cast it to text.
    const ICAL_FEED_COLUMNS: &'static str = r#"
        id, organization_id, unit_id, feed_name, feed_token, feed_url,
        import_url, import_platform::text AS import_platform,
        last_import_at, import_error, is_active, created_at, updated_at
    "#;

    // ========================================================================
    // Platform Connections (Story 18.1)
    // ========================================================================

    /// Create platform connection.
    pub async fn create_connection(
        &self,
        org_id: Uuid,
        data: CreatePlatformConnection,
    ) -> Result<RentalPlatformConnection, SqlxError> {
        // PAP-158: RETURNING * would decode the `platform` enum into the model's
        // `String` field and panic; project the columns with `platform::text`.
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_platform_connections (
                organization_id, unit_id, platform, external_property_id,
                sync_calendar, sync_interval_minutes, block_other_platforms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING {}
            "#,
            Self::CONNECTION_COLUMNS
        )))
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
        // `platform` is the `rental_platform` enum; the model decodes it as
        // `String`, so a bare `SELECT *` panics with "mismatched types … not
        // compatible with SQL type rental_platform". Cast the column to text on
        // read and the bound text arg to the enum in the predicate.
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
            WHERE unit_id = $1 AND platform = $2::rental_platform
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
        // PAP-158: cast the `platform` enum to text in RETURNING (see
        // CONNECTION_COLUMNS) so the row decodes into the `String` model field.
        let conn = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
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
            RETURNING {}
            "#,
            Self::CONNECTION_COLUMNS
        )))
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
                c.id, c.unit_id, u.name as unit_name,
                c.platform::text, c.is_active, c.last_sync_at, c.sync_error
            FROM rental_platform_connections c
            JOIN units u ON u.id = c.unit_id
            WHERE c.organization_id = $1
            ORDER BY u.name, c.platform
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

    // ========================================================================
    // Bookings (Story 18.2)
    // ========================================================================

    /// Create booking.
    pub async fn create_booking(
        &self,
        org_id: Uuid,
        data: CreateBooking,
    ) -> Result<RentalBooking, SqlxError> {
        // PAP-158: RETURNING * would decode the `platform`/`status` enums into
        // the model's `String` fields and panic; project with enum-to-text casts.
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_bookings (
                organization_id, unit_id, platform, external_booking_id,
                guest_name, guest_email, guest_phone, guest_count,
                check_in, check_out, check_in_time, check_out_time,
                total_amount, currency, platform_fee, cleaning_fee,
                guest_notes, internal_notes, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            RETURNING {}
            "#,
            Self::BOOKING_COLUMNS
        )))
        .bind(org_id)
        .bind(data.unit_id)
        .bind(&data.platform)
        .bind(&data.external_booking_id)
        .bind(&data.guest_name)
        .bind(&data.guest_email)
        .bind(&data.guest_phone)
        .bind(data.guest_count)
        .bind(data.check_in)
        .bind(data.check_out)
        .bind(data.check_in_time)
        .bind(data.check_out_time)
        .bind(data.total_amount)
        .bind(&data.currency)
        .bind(data.platform_fee)
        .bind(data.cleaning_fee)
        .bind(&data.guest_notes)
        .bind(&data.internal_notes)
        .bind(booking_status::PENDING)
        .fetch_one(&self.pool)
        .await?;

        // Create calendar block for the booking
        sqlx::query(
            r#"
            INSERT INTO rental_calendar_blocks (organization_id, unit_id, block_start, block_end, reason, booking_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(data.check_in)
        .bind(data.check_out)
        .bind(block_reason::BOOKING)
        .bind(booking.id)
        .execute(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Find booking by ID.
    pub async fn find_booking_by_id(&self, id: Uuid) -> Result<Option<RentalBooking>, SqlxError> {
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_bookings WHERE id = $1",
            Self::BOOKING_COLUMNS
        )))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Find booking by ID scoped to an organization.
    ///
    /// SECURITY (#804): prevents reading another org's booking PII by UUID.
    pub async fn find_booking_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<RentalBooking>, SqlxError> {
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_bookings WHERE id = $1 AND organization_id = $2",
            Self::BOOKING_COLUMNS
        )))
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Find booking by external ID.
    pub async fn find_booking_by_external_id(
        &self,
        platform: &str,
        external_id: &str,
    ) -> Result<Option<RentalBooking>, SqlxError> {
        // `platform`/`status` are PG enums but the model decodes them as
        // `String`; a bare `SELECT *` fails to decode (42804, FORCE-RLS-masked,
        // PAP-158), so cast both enum columns to text. `platform = $1` keeps the
        // text param comparing against the enum via the column's implicit cast.
        let booking = sqlx::query_as::<_, RentalBooking>(
            r#"
            SELECT
                id, organization_id, unit_id, connection_id,
                platform::text AS platform, external_booking_id, external_booking_url,
                guest_name, guest_email, guest_phone, guest_count,
                check_in, check_out, check_in_time, check_out_time,
                total_amount, currency, platform_fee, cleaning_fee,
                status::text AS status, cancelled_at, cancellation_reason,
                guest_notes, internal_notes, synced_at, raw_data,
                created_at, updated_at
            FROM rental_bookings
            WHERE platform = $1::rental_platform AND external_booking_id = $2
            "#,
        )
        .bind(platform)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Update booking.
    pub async fn update_booking(
        &self,
        id: Uuid,
        data: UpdateBooking,
    ) -> Result<RentalBooking, SqlxError> {
        // PAP-158: project enum columns as text in RETURNING (see BOOKING_COLUMNS).
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_bookings SET
                guest_name = COALESCE($2, guest_name),
                guest_email = COALESCE($3, guest_email),
                guest_phone = COALESCE($4, guest_phone),
                guest_count = COALESCE($5, guest_count),
                check_in = COALESCE($6, check_in),
                check_out = COALESCE($7, check_out),
                check_in_time = COALESCE($8, check_in_time),
                check_out_time = COALESCE($9, check_out_time),
                total_amount = COALESCE($10, total_amount),
                currency = COALESCE($11, currency),
                guest_notes = COALESCE($12, guest_notes),
                internal_notes = COALESCE($13, internal_notes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {}
            "#,
            Self::BOOKING_COLUMNS
        )))
        .bind(id)
        .bind(&data.guest_name)
        .bind(&data.guest_email)
        .bind(&data.guest_phone)
        .bind(data.guest_count)
        .bind(data.check_in)
        .bind(data.check_out)
        .bind(data.check_in_time)
        .bind(data.check_out_time)
        .bind(data.total_amount)
        .bind(&data.currency)
        .bind(&data.guest_notes)
        .bind(&data.internal_notes)
        .fetch_one(&self.pool)
        .await?;

        // Update calendar block if dates changed
        if data.check_in.is_some() || data.check_out.is_some() {
            sqlx::query(
                r#"
                UPDATE rental_calendar_blocks SET
                    block_start = COALESCE($2, block_start),
                    block_end = COALESCE($3, block_end)
                WHERE booking_id = $1
                "#,
            )
            .bind(id)
            .bind(data.check_in)
            .bind(data.check_out)
            .execute(&self.pool)
            .await?;
        }

        Ok(booking)
    }

    /// Update booking scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $14` guard prevents a tenant
    /// from mutating another org's booking. Returns `None` when no row matched.
    pub async fn update_booking_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        data: UpdateBooking,
    ) -> Result<Option<RentalBooking>, SqlxError> {
        // PAP-158: project enum columns as text in RETURNING (see BOOKING_COLUMNS).
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_bookings SET
                guest_name = COALESCE($2, guest_name),
                guest_email = COALESCE($3, guest_email),
                guest_phone = COALESCE($4, guest_phone),
                guest_count = COALESCE($5, guest_count),
                check_in = COALESCE($6, check_in),
                check_out = COALESCE($7, check_out),
                check_in_time = COALESCE($8, check_in_time),
                check_out_time = COALESCE($9, check_out_time),
                total_amount = COALESCE($10, total_amount),
                currency = COALESCE($11, currency),
                guest_notes = COALESCE($12, guest_notes),
                internal_notes = COALESCE($13, internal_notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $14
            RETURNING {}
            "#,
            Self::BOOKING_COLUMNS
        )))
        .bind(id)
        .bind(&data.guest_name)
        .bind(&data.guest_email)
        .bind(&data.guest_phone)
        .bind(data.guest_count)
        .bind(data.check_in)
        .bind(data.check_out)
        .bind(data.check_in_time)
        .bind(data.check_out_time)
        .bind(data.total_amount)
        .bind(&data.currency)
        .bind(&data.guest_notes)
        .bind(&data.internal_notes)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        // Only touch the calendar block if the booking belonged to this org.
        if booking.is_some() && (data.check_in.is_some() || data.check_out.is_some()) {
            sqlx::query(
                r#"
                UPDATE rental_calendar_blocks SET
                    block_start = COALESCE($2, block_start),
                    block_end = COALESCE($3, block_end)
                WHERE booking_id = $1
                "#,
            )
            .bind(id)
            .bind(data.check_in)
            .bind(data.check_out)
            .execute(&self.pool)
            .await?;
        }

        Ok(booking)
    }

    /// Update booking status scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $4` guard prevents a tenant
    /// from changing another org's booking status. Returns `None` if no match.
    pub async fn update_booking_status_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        data: UpdateBookingStatus,
    ) -> Result<Option<RentalBooking>, SqlxError> {
        // `status`/`platform` are PG enums but the model decodes them as
        // `String`. The status param `$2` is used both as an enum assignment
        // AND in the `CASE WHEN $2 = 'cancelled'` text comparison; without the
        // explicit `$2::rental_booking_status` cast the comparison pins `$2` to
        // `text`, so the assignment fails with 42804 (FORCE-RLS-masked, see
        // PAP-158). `RETURNING *` likewise must cast the enum columns to text or
        // the row fails to decode into `RentalBooking`.
        let booking = sqlx::query_as::<_, RentalBooking>(
            r#"
            UPDATE rental_bookings SET
                status = $2::rental_booking_status,
                cancelled_at = CASE WHEN $2 = 'cancelled' THEN NOW() ELSE cancelled_at END,
                cancellation_reason = COALESCE($3, cancellation_reason),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $4
            RETURNING
                id, organization_id, unit_id, connection_id,
                platform::text AS platform, external_booking_id, external_booking_url,
                guest_name, guest_email, guest_phone, guest_count,
                check_in, check_out, check_in_time, check_out_time,
                total_amount, currency, platform_fee, cleaning_fee,
                status::text AS status, cancelled_at, cancellation_reason,
                guest_notes, internal_notes, synced_at, raw_data,
                created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.status)
        .bind(&data.cancellation_reason)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        // Remove calendar block if cancelled (only when the booking was ours).
        if booking.is_some() && data.status == booking_status::CANCELLED {
            sqlx::query(r#"DELETE FROM rental_calendar_blocks WHERE booking_id = $1"#)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        Ok(booking)
    }

    /// List bookings with filters.
    pub async fn list_bookings(
        &self,
        org_id: Uuid,
        query: BookingListQuery,
    ) -> Result<(Vec<BookingSummary>, i64), SqlxError> {
        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        // Build dynamic query
        let mut conditions = vec!["b.organization_id = $1".to_string()];
        let mut param_count = 1;

        if query.unit_id.is_some() {
            param_count += 1;
            conditions.push(format!("b.unit_id = ${}", param_count));
        }
        if query.building_id.is_some() {
            param_count += 1;
            conditions.push(format!("u.building_id = ${}", param_count));
        }
        if query.platform.is_some() {
            param_count += 1;
            // PAP-158: `platform` is the `rental_platform` enum; comparing it to a
            // bound text param needs an explicit `::text` cast (else 42883).
            conditions.push(format!("b.platform::text = ${}", param_count));
        }
        if query.status.is_some() {
            param_count += 1;
            // PAP-158: `status` is the `rental_booking_status` enum; cast to text
            // for the text-param comparison.
            conditions.push(format!("b.status::text = ${}", param_count));
        }
        if query.from_date.is_some() {
            param_count += 1;
            conditions.push(format!("b.check_in >= ${}", param_count));
        }
        if query.to_date.is_some() {
            param_count += 1;
            conditions.push(format!("b.check_out <= ${}", param_count));
        }
        if query.guest_name.is_some() {
            param_count += 1;
            conditions.push(format!("b.guest_name ILIKE '%' || ${} || '%'", param_count));
        }

        let where_clause = conditions.join(" AND ");

        // Count total
        let count_query = format!(
            r#"
            SELECT COUNT(*)
            FROM rental_bookings b
            JOIN units u ON u.id = b.unit_id
            JOIN buildings bld ON bld.id = u.building_id
            WHERE {}
            "#,
            where_clause
        );

        let mut count_builder =
            sqlx::query_as::<_, (i64,)>(sqlx::AssertSqlSafe(count_query)).bind(org_id);

        if let Some(unit_id) = query.unit_id {
            count_builder = count_builder.bind(unit_id);
        }
        if let Some(building_id) = query.building_id {
            count_builder = count_builder.bind(building_id);
        }
        if let Some(ref platform) = query.platform {
            count_builder = count_builder.bind(platform);
        }
        if let Some(ref status) = query.status {
            count_builder = count_builder.bind(status);
        }
        if let Some(from_date) = query.from_date {
            count_builder = count_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            count_builder = count_builder.bind(to_date);
        }
        if let Some(ref guest_name) = query.guest_name {
            count_builder = count_builder.bind(guest_name);
        }

        let (total,) = count_builder.fetch_one(&self.pool).await?;

        // Fetch bookings (simplified - using direct query)
        let bookings = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, String, i32, NaiveDate, NaiveDate, Option<Decimal>, Option<String>, String, Option<String>)>(
            r#"
            SELECT
                b.id, b.unit_id, u.name, bld.name,
                b.platform::text, b.external_booking_id, b.guest_name, b.guest_count,
                b.check_in, b.check_out, b.total_amount, b.currency,
                b.status::text,
                (SELECT status::text FROM rental_guests WHERE booking_id = b.id AND is_primary = true LIMIT 1)
            FROM rental_bookings b
            JOIN units u ON u.id = b.unit_id
            JOIN buildings bld ON bld.id = u.building_id
            WHERE b.organization_id = $1
            ORDER BY b.check_in DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let summaries = bookings
            .into_iter()
            .map(
                |(
                    id,
                    unit_id,
                    unit_name,
                    building_name,
                    platform,
                    external_booking_id,
                    guest_name,
                    guest_count,
                    check_in,
                    check_out,
                    total_amount,
                    currency,
                    status,
                    guest_status,
                )| {
                    BookingSummary {
                        id,
                        unit_id,
                        unit_name,
                        building_name,
                        platform,
                        external_booking_id,
                        guest_name,
                        guest_count,
                        check_in,
                        check_out,
                        nights: (check_out - check_in).num_days(),
                        total_amount,
                        currency,
                        status,
                        guest_registration_status: guest_status,
                    }
                },
            )
            .collect();

        Ok((summaries, total))
    }

    /// Get upcoming check-ins needing guest registration.
    pub async fn get_upcoming_checkins_needing_registration(
        &self,
        org_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<CheckInReminder>, SqlxError> {
        let today = Utc::now().date_naive();
        let target_date = today + Duration::days(days_ahead as i64);

        let reminders = sqlx::query_as::<_, (Uuid, String, String, NaiveDate, i64)>(
            r#"
            SELECT
                b.id, u.name, b.guest_name, b.check_in,
                (SELECT COUNT(*) FROM rental_guests WHERE booking_id = b.id AND status = 'pending')
            FROM rental_bookings b
            JOIN units u ON u.id = b.unit_id
            WHERE b.organization_id = $1
                AND b.status IN ('pending', 'confirmed')
                AND b.check_in BETWEEN $2 AND $3
                AND EXISTS (SELECT 1 FROM rental_guests WHERE booking_id = b.id AND status = 'pending')
            ORDER BY b.check_in
            "#,
        )
        .bind(org_id)
        .bind(today)
        .bind(target_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(reminders
            .into_iter()
            .map(
                |(booking_id, unit_name, guest_name, check_in, pending)| CheckInReminder {
                    booking_id,
                    unit_name,
                    guest_name,
                    check_in,
                    pending_registrations: pending as i32,
                },
            )
            .collect())
    }

    // ========================================================================
    // Calendar (Story 18.2)
    // ========================================================================

    /// Create calendar block.
    pub async fn create_calendar_block(
        &self,
        org_id: Uuid,
        data: CreateCalendarBlock,
    ) -> Result<CalendarBlock, SqlxError> {
        // PAP-158: RETURNING * would decode the nullable `source_platform` enum
        // into `Option<String>` and panic; project with enum-to-text cast.
        let block = sqlx::query_as::<_, CalendarBlock>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_calendar_blocks (organization_id, unit_id, block_start, block_end, reason, notes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING {}
            "#,
            Self::CALENDAR_BLOCK_COLUMNS
        )))
        .bind(org_id)
        .bind(data.unit_id)
        .bind(data.block_start)
        .bind(data.block_end)
        .bind(&data.reason)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(block)
    }

    /// Delete calendar block.
    pub async fn delete_calendar_block(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"DELETE FROM rental_calendar_blocks WHERE id = $1 AND booking_id IS NULL"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete calendar block scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $2` guard prevents a tenant
    /// from deleting another org's calendar block.
    pub async fn delete_calendar_block_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"DELETE FROM rental_calendar_blocks WHERE id = $1 AND organization_id = $2 AND booking_id IS NULL"#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Verify a unit belongs to an organization.
    ///
    /// SECURITY (#804): used by unit-scoped read endpoints (calendar,
    /// availability) so a caller cannot probe another org's unit by UUID.
    pub async fn unit_belongs_to_org(
        &self,
        org_id: Uuid,
        unit_id: Uuid,
    ) -> Result<bool, SqlxError> {
        // `units` has no `organization_id` column — org is reached through
        // `buildings.organization_id` (see migration 00116 / 00140 RLS notes).
        let (exists,): (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM units u
                JOIN buildings b ON b.id = u.building_id
                WHERE u.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    /// Get calendar events for unit in date range.
    pub async fn get_calendar_events(
        &self,
        unit_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<CalendarEvent>, SqlxError> {
        // Get blocks
        let blocks = sqlx::query_as::<
            _,
            (
                Uuid,
                NaiveDate,
                NaiveDate,
                String,
                Option<Uuid>,
                Option<String>,
            ),
        >(
            r#"
            SELECT id, block_start, block_end, reason, booking_id, source_platform::text
            FROM rental_calendar_blocks
            WHERE unit_id = $1 AND block_start <= $3 AND block_end >= $2
            ORDER BY block_start
            "#,
        )
        .bind(unit_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        // Get bookings for those blocks
        let mut events: Vec<CalendarEvent> = Vec::new();

        for (id, block_start, block_end, reason, booking_id, source_platform) in blocks {
            let (title, booking_status, color) = if let Some(bid) = booking_id {
                // Get booking info
                // PAP-158: `status` is the `rental_booking_status` enum decoded
                // into a `String` tuple field; cast it to text.
                let booking: Option<(String, String, String)> = sqlx::query_as(
                    r#"SELECT guest_name, platform::text, status::text FROM rental_bookings WHERE id = $1"#,
                )
                .bind(bid)
                .fetch_optional(&self.pool)
                .await?;

                if let Some((guest_name, _platform, status)) = booking {
                    let color = match status.as_str() {
                        "confirmed" => "#22c55e",
                        "checked_in" => "#3b82f6",
                        "pending" => "#f59e0b",
                        _ => "#6b7280",
                    };
                    (guest_name, Some(status), color.to_string())
                } else {
                    ("Booking".to_string(), None, "#6b7280".to_string())
                }
            } else {
                let title = match reason.as_str() {
                    "maintenance" => "Maintenance",
                    "owner_use" => "Owner Use",
                    _ => "Blocked",
                };
                let color = match reason.as_str() {
                    "maintenance" => "#ef4444",
                    "owner_use" => "#8b5cf6",
                    _ => "#6b7280",
                };
                (title.to_string(), None, color.to_string())
            };

            events.push(CalendarEvent {
                id,
                unit_id,
                start_date: block_start,
                end_date: block_end,
                event_type: if booking_id.is_some() {
                    "booking".to_string()
                } else {
                    "block".to_string()
                },
                title,
                platform: source_platform,
                booking_status,
                color,
            });
        }

        Ok(events)
    }

    /// Check availability for unit.
    pub async fn check_availability(
        &self,
        unit_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<bool, SqlxError> {
        let (has_conflict,): (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM rental_calendar_blocks
                WHERE unit_id = $1
                    AND block_start < $3
                    AND block_end > $2
            )
            "#,
        )
        .bind(unit_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(!has_conflict)
    }

    // ========================================================================
    // Guests (Story 18.3)
    // ========================================================================

    /// Create guest.
    pub async fn create_guest(
        &self,
        org_id: Uuid,
        data: CreateGuest,
    ) -> Result<RentalGuest, SqlxError> {
        // PAP-158: RETURNING * would decode the `status` enum into the model's
        // `String` field and panic; project with enum-to-text cast.
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_guests (
                organization_id, booking_id, first_name, last_name,
                date_of_birth, nationality, id_type, id_number,
                id_issuing_country, id_expiry_date, email, phone,
                address_street, address_city, address_postal_code, address_country,
                is_primary, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING {}
            "#,
            Self::GUEST_COLUMNS
        )))
        .bind(org_id)
        .bind(data.booking_id)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(data.date_of_birth)
        .bind(&data.nationality)
        .bind(&data.id_type)
        .bind(&data.id_number)
        .bind(&data.id_issuing_country)
        .bind(data.id_expiry_date)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.address_street)
        .bind(&data.address_city)
        .bind(&data.address_postal_code)
        .bind(&data.address_country)
        .bind(data.is_primary)
        .bind(guest_status::PENDING)
        .fetch_one(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Find guest by ID.
    pub async fn find_guest_by_id(&self, id: Uuid) -> Result<Option<RentalGuest>, SqlxError> {
        // PAP-158: `status` is an enum; project with enum-to-text cast.
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_guests WHERE id = $1",
            Self::GUEST_COLUMNS
        )))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Find guest by ID scoped to an organization.
    ///
    /// SECURITY (#804): prevents reading another org's guest PII by UUID.
    pub async fn find_guest_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<RentalGuest>, SqlxError> {
        // PAP-158: `status` is an enum; project with enum-to-text cast.
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_guests WHERE id = $1 AND organization_id = $2",
            Self::GUEST_COLUMNS
        )))
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Update guest.
    pub async fn update_guest(
        &self,
        id: Uuid,
        data: UpdateGuest,
    ) -> Result<RentalGuest, SqlxError> {
        // PAP-158: project `status` enum as text in RETURNING (see GUEST_COLUMNS).
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_guests SET
                first_name = COALESCE($2, first_name),
                last_name = COALESCE($3, last_name),
                date_of_birth = COALESCE($4, date_of_birth),
                nationality = COALESCE($5, nationality),
                id_type = COALESCE($6, id_type),
                id_number = COALESCE($7, id_number),
                id_issuing_country = COALESCE($8, id_issuing_country),
                id_expiry_date = COALESCE($9, id_expiry_date),
                id_document_url = COALESCE($10, id_document_url),
                email = COALESCE($11, email),
                phone = COALESCE($12, phone),
                address_street = COALESCE($13, address_street),
                address_city = COALESCE($14, address_city),
                address_postal_code = COALESCE($15, address_postal_code),
                address_country = COALESCE($16, address_country),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {}
            "#,
            Self::GUEST_COLUMNS
        )))
        .bind(id)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(data.date_of_birth)
        .bind(&data.nationality)
        .bind(&data.id_type)
        .bind(&data.id_number)
        .bind(&data.id_issuing_country)
        .bind(data.id_expiry_date)
        .bind(&data.id_document_url)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.address_street)
        .bind(&data.address_city)
        .bind(&data.address_postal_code)
        .bind(&data.address_country)
        .fetch_one(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Update guest scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $17` guard prevents a tenant
    /// from mutating another org's guest. Returns `None` when no row matched.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_guest_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        data: UpdateGuest,
    ) -> Result<Option<RentalGuest>, SqlxError> {
        // PAP-158: project `status` enum as text in RETURNING (see GUEST_COLUMNS).
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_guests SET
                first_name = COALESCE($2, first_name),
                last_name = COALESCE($3, last_name),
                date_of_birth = COALESCE($4, date_of_birth),
                nationality = COALESCE($5, nationality),
                id_type = COALESCE($6, id_type),
                id_number = COALESCE($7, id_number),
                id_issuing_country = COALESCE($8, id_issuing_country),
                id_expiry_date = COALESCE($9, id_expiry_date),
                id_document_url = COALESCE($10, id_document_url),
                email = COALESCE($11, email),
                phone = COALESCE($12, phone),
                address_street = COALESCE($13, address_street),
                address_city = COALESCE($14, address_city),
                address_postal_code = COALESCE($15, address_postal_code),
                address_country = COALESCE($16, address_country),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $17
            RETURNING {}
            "#,
            Self::GUEST_COLUMNS
        )))
        .bind(id)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(data.date_of_birth)
        .bind(&data.nationality)
        .bind(&data.id_type)
        .bind(&data.id_number)
        .bind(&data.id_issuing_country)
        .bind(data.id_expiry_date)
        .bind(&data.id_document_url)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.address_street)
        .bind(&data.address_city)
        .bind(&data.address_postal_code)
        .bind(&data.address_country)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Register guest (mark as registered).
    pub async fn register_guest(&self, id: Uuid) -> Result<RentalGuest, SqlxError> {
        // PAP-158: cast the bound text status arg to the enum on assignment and
        // project `status` as text in RETURNING (see GUEST_COLUMNS).
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_guests SET
                status = $2::guest_registration_status,
                registered_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {}
            "#,
            Self::GUEST_COLUMNS
        )))
        .bind(id)
        .bind(guest_status::REGISTERED)
        .fetch_one(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Register guest scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $3` guard prevents a tenant
    /// from registering another org's guest. Returns `None` when no row matched.
    pub async fn register_guest_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<RentalGuest>, SqlxError> {
        // PAP-158: cast the bound text status arg to the enum on assignment and
        // project `status` as text in RETURNING (see GUEST_COLUMNS).
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_guests SET
                status = $2::guest_registration_status,
                registered_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $3
            RETURNING {}
            "#,
            Self::GUEST_COLUMNS
        )))
        .bind(id)
        .bind(guest_status::REGISTERED)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(guest)
    }

    /// Get guests for booking.
    pub async fn get_guests_for_booking(
        &self,
        booking_id: Uuid,
    ) -> Result<Vec<RentalGuest>, SqlxError> {
        // PAP-158: `status` is an enum; project with enum-to-text cast.
        let guests = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {} FROM rental_guests
            WHERE booking_id = $1
            ORDER BY is_primary DESC, created_at
            "#,
            Self::GUEST_COLUMNS
        )))
        .bind(booking_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(guests)
    }

    /// Get booking with guests.
    pub async fn get_booking_with_guests(
        &self,
        booking_id: Uuid,
    ) -> Result<Option<BookingWithGuests>, SqlxError> {
        let booking = self.find_booking_by_id(booking_id).await?;
        if booking.is_none() {
            return Ok(None);
        }
        let booking = booking.unwrap();

        let guests = self.get_guests_for_booking(booking_id).await?;

        // Check if registration is complete (all guests registered)
        let registration_complete = !guests.is_empty()
            && guests.iter().all(|g| {
                g.status == guest_status::REGISTERED || g.status == guest_status::REPORTED
            });

        Ok(Some(BookingWithGuests {
            booking,
            guests,
            registration_complete,
        }))
    }

    /// Get booking with guests scoped to an organization.
    ///
    /// SECURITY (#804): resolves the booking via the org-scoped lookup so a
    /// caller cannot read another org's booking + guest PII by UUID.
    pub async fn get_booking_with_guests_for_org(
        &self,
        org_id: Uuid,
        booking_id: Uuid,
    ) -> Result<Option<BookingWithGuests>, SqlxError> {
        let booking = match self.find_booking_for_org(org_id, booking_id).await? {
            Some(b) => b,
            None => return Ok(None),
        };

        let guests = self.get_guests_for_booking(booking_id).await?;

        let registration_complete = !guests.is_empty()
            && guests.iter().all(|g| {
                g.status == guest_status::REGISTERED || g.status == guest_status::REPORTED
            });

        Ok(Some(BookingWithGuests {
            booking,
            guests,
            registration_complete,
        }))
    }

    /// Delete guest scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $2` guard prevents a tenant
    /// from deleting another org's guest.
    pub async fn delete_guest_for_org(&self, org_id: Uuid, id: Uuid) -> Result<bool, SqlxError> {
        let result =
            sqlx::query(r#"DELETE FROM rental_guests WHERE id = $1 AND organization_id = $2"#)
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete guest.
    pub async fn delete_guest(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM rental_guests WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Reports (Story 18.4)
    // ========================================================================

    /// Generate report preview.
    pub async fn generate_report_preview(
        &self,
        org_id: Uuid,
        building_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<ReportPreview, SqlxError> {
        // Get building name
        let (building_name,): (String,) =
            sqlx::query_as(r#"SELECT name FROM buildings WHERE id = $1"#)
                .bind(building_id)
                .fetch_one(&self.pool)
                .await?;

        // Count guests and get nationality breakdown
        let stats = sqlx::query_as::<_, (i64, Option<String>)>(
            r#"
            SELECT COUNT(g.id), g.nationality
            FROM rental_guests g
            JOIN rental_bookings b ON b.id = g.booking_id
            JOIN units u ON u.id = b.unit_id
            WHERE b.organization_id = $1
                AND u.building_id = $2
                AND b.check_in >= $3
                AND b.check_out <= $4
                AND g.status IN ('registered', 'reported')
            GROUP BY g.nationality
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await?;

        let total_guests: i32 = stats.iter().map(|(count, _)| *count as i32).sum();

        let by_nationality: Vec<NationalityStats> = stats
            .into_iter()
            .map(|(count, nationality)| {
                let nat = nationality.unwrap_or_else(|| "UNK".to_string());
                NationalityStats {
                    nationality: nat.clone(),
                    country_name: get_country_name(&nat),
                    count: count as i32,
                    percentage: if total_guests > 0 {
                        (count as f64 / total_guests as f64) * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect();

        // Count bookings
        let (bookings_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT b.id)
            FROM rental_bookings b
            JOIN units u ON u.id = b.unit_id
            WHERE b.organization_id = $1
                AND u.building_id = $2
                AND b.check_in >= $3
                AND b.check_out <= $4
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        Ok(ReportPreview {
            building_id,
            building_name,
            period_start,
            period_end,
            total_guests,
            by_nationality,
            bookings_count: bookings_count as i32,
        })
    }

    /// Create report.
    pub async fn create_report(
        &self,
        org_id: Uuid,
        data: GenerateReport,
        user_id: Uuid,
    ) -> Result<RentalGuestReport, SqlxError> {
        // Get preview data
        let preview = self
            .generate_report_preview(org_id, data.building_id, data.period_start, data.period_end)
            .await?;

        let authority_name = match data.authority_code.as_str() {
            "SK_UHUL" => "UHUL Slovakia",
            "CZ_CIZPOL" => "Czech Foreign Police",
            "AT_ZMR" => "Austria ZMR",
            "DE_MELDEWESEN" => "Germany Meldewesen",
            _ => "Unknown",
        };

        let guests_by_nationality = serde_json::to_value(&preview.by_nationality).ok();

        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"
            INSERT INTO rental_guest_reports (
                organization_id, building_id, report_type, period_start, period_end,
                authority_code, authority_name, total_guests, guests_by_nationality,
                report_format, status, generated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.report_type)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(&data.authority_code)
        .bind(authority_name)
        .bind(preview.total_guests)
        .bind(guests_by_nationality)
        .bind(&data.report_format)
        .bind(report_status::GENERATED)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(report)
    }

    /// Find report by ID.
    pub async fn find_report_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<RentalGuestReport>, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"SELECT * FROM rental_guest_reports WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(report)
    }

    /// Find report by ID scoped to an organization.
    ///
    /// SECURITY (#804): prevents reading another org's authority report by UUID.
    pub async fn find_report_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<RentalGuestReport>, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"SELECT * FROM rental_guest_reports WHERE id = $1 AND organization_id = $2"#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(report)
    }

    /// Submit report scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $4` guard prevents a tenant
    /// from submitting another org's report. Returns `None` when no row matched.
    pub async fn submit_report_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<RentalGuestReport>, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"
            UPDATE rental_guest_reports SET
                status = $2,
                submitted_at = NOW(),
                submitted_by = $3,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $4
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(report_status::SUBMITTED)
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        // Mark guests as reported only when the report belonged to this org.
        if report.is_some() {
            sqlx::query(
                r#"
                UPDATE rental_guests SET
                    status = 'reported',
                    reported_at = NOW()
                WHERE booking_id IN (
                    SELECT b.id FROM rental_bookings b
                    JOIN units u ON u.id = b.unit_id
                    JOIN rental_guest_reports r ON r.building_id = u.building_id
                    WHERE r.id = $1
                        AND b.check_in >= r.period_start
                        AND b.check_out <= r.period_end
                )
                AND status = 'registered'
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(report)
    }

    /// Submit report.
    pub async fn submit_report(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<RentalGuestReport, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"
            UPDATE rental_guest_reports SET
                status = $2,
                submitted_at = NOW(),
                submitted_by = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(report_status::SUBMITTED)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Mark guests as reported
        sqlx::query(
            r#"
            UPDATE rental_guests SET
                status = 'reported',
                reported_at = NOW()
            WHERE booking_id IN (
                SELECT b.id FROM rental_bookings b
                JOIN units u ON u.id = b.unit_id
                JOIN rental_guest_reports r ON r.building_id = u.building_id
                WHERE r.id = $1
                    AND b.check_in >= r.period_start
                    AND b.check_out <= r.period_end
            )
            AND status = 'registered'
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(report)
    }

    /// List reports for organization.
    pub async fn list_reports(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
        limit: i32,
    ) -> Result<Vec<ReportSummary>, SqlxError> {
        let reports = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                NaiveDate,
                NaiveDate,
                String,
                String,
                i32,
                String,
                Option<String>,
                Option<chrono::DateTime<Utc>>,
            ),
        >(
            r#"
            SELECT
                r.id, r.building_id, b.name, r.report_type,
                r.period_start, r.period_end, r.authority_code, r.authority_name,
                r.total_guests, r.status, r.report_file_url, r.submitted_at
            FROM rental_guest_reports r
            JOIN buildings b ON b.id = r.building_id
            WHERE r.organization_id = $1
                AND ($2::uuid IS NULL OR r.building_id = $2)
            ORDER BY r.period_start DESC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(reports
            .into_iter()
            .map(
                |(
                    id,
                    building_id,
                    building_name,
                    report_type,
                    period_start,
                    period_end,
                    authority_code,
                    authority_name,
                    total_guests,
                    status,
                    report_file_url,
                    submitted_at,
                )| {
                    ReportSummary {
                        id,
                        building_id,
                        building_name,
                        report_type,
                        period_start,
                        period_end,
                        authority_code,
                        authority_name,
                        total_guests,
                        status,
                        report_file_url,
                        submitted_at,
                    }
                },
            )
            .collect())
    }

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

        // PAP-158: RETURNING * would decode the nullable `import_platform` enum
        // into `Option<String>` and panic; project with enum-to-text cast.
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_ical_feeds (
                organization_id, unit_id, feed_name, feed_token,
                import_url, import_platform
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING {}
            "#,
            Self::ICAL_FEED_COLUMNS
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
        // PAP-158: `import_platform` is an enum; project with enum-to-text cast.
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_ical_feeds WHERE feed_token = $1 AND is_active = true",
            Self::ICAL_FEED_COLUMNS
        )))
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(feed)
    }

    /// Get iCal feeds for unit.
    pub async fn get_ical_feeds_for_unit(&self, unit_id: Uuid) -> Result<Vec<ICalFeed>, SqlxError> {
        // PAP-158: `import_platform` is an enum; project with enum-to-text cast.
        let feeds = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_ical_feeds WHERE unit_id = $1 ORDER BY feed_name",
            Self::ICAL_FEED_COLUMNS
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
        // PAP-158: `import_platform` is an enum; project with enum-to-text cast.
        let feeds = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            "SELECT {} FROM rental_ical_feeds WHERE unit_id = $1 AND organization_id = $2 ORDER BY feed_name",
            Self::ICAL_FEED_COLUMNS
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
        // PAP-158: project `import_platform` enum as text (see ICAL_FEED_COLUMNS).
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_ical_feeds SET
                feed_name = COALESCE($2, feed_name),
                import_url = COALESCE($3, import_url),
                is_active = COALESCE($4, is_active),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $5
            RETURNING {}
            "#,
            Self::ICAL_FEED_COLUMNS
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
        // PAP-158: project `import_platform` enum as text (see ICAL_FEED_COLUMNS).
        let feed = sqlx::query_as::<_, ICalFeed>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_ical_feeds SET
                feed_name = COALESCE($2, feed_name),
                import_url = COALESCE($3, import_url),
                is_active = COALESCE($4, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {}
            "#,
            Self::ICAL_FEED_COLUMNS
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

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get rental statistics for organization.
    pub async fn get_statistics(&self, org_id: Uuid) -> Result<RentalStatistics, SqlxError> {
        // Get unit counts
        let (total_units,): (i64,) =
            sqlx::query_as(r#"SELECT COUNT(DISTINCT id) FROM units WHERE organization_id = $1"#)
                .bind(org_id)
                .fetch_one(&self.pool)
                .await?;

        let (connected_units,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT unit_id) FROM rental_platform_connections WHERE organization_id = $1 AND is_active = true"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Get booking counts
        let today = Utc::now().date_naive();
        let (active_bookings,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rental_bookings
            WHERE organization_id = $1
                AND status IN ('confirmed', 'checked_in')
                AND check_in <= $2 AND check_out >= $2
            "#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        let (upcoming_bookings,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rental_bookings
            WHERE organization_id = $1
                AND status IN ('pending', 'confirmed')
                AND check_in > $2
            "#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        let (pending_registrations,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM rental_guests WHERE organization_id = $1 AND status = 'pending'"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Calculate occupancy (simplified)
        let occupancy_rate = if total_units > 0 {
            (active_bookings as f64 / total_units as f64) * 100.0
        } else {
            0.0
        };

        // Revenue calculations (simplified)
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let last_month_start = month_start - Duration::days(30);

        let (revenue_this_month,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(total_amount), 0)
            FROM rental_bookings
            WHERE organization_id = $1
                AND status NOT IN ('cancelled', 'no_show')
                AND check_in >= $2
            "#,
        )
        .bind(org_id)
        .bind(month_start)
        .fetch_one(&self.pool)
        .await?;

        let (revenue_last_month,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(total_amount), 0)
            FROM rental_bookings
            WHERE organization_id = $1
                AND status NOT IN ('cancelled', 'no_show')
                AND check_in >= $2 AND check_in < $3
            "#,
        )
        .bind(org_id)
        .bind(last_month_start)
        .bind(month_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(RentalStatistics {
            total_units,
            connected_units,
            active_bookings,
            upcoming_bookings,
            pending_registrations,
            occupancy_rate,
            revenue_this_month: revenue_this_month.unwrap_or_default(),
            revenue_last_month: revenue_last_month.unwrap_or_default(),
        })
    }

    /// Get platform sync status.
    pub async fn get_platform_sync_status(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PlatformSyncStatus>, SqlxError> {
        let statuses = sqlx::query_as::<_, (String, i64, Option<chrono::DateTime<Utc>>, i64)>(
            r#"
            SELECT
                platform::text,
                COUNT(*) as connections_count,
                MAX(last_sync_at) as last_sync,
                COUNT(*) FILTER (WHERE sync_error IS NOT NULL) as errors
            FROM rental_platform_connections
            WHERE organization_id = $1 AND is_active = true
            GROUP BY platform
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(statuses
            .into_iter()
            .map(|(platform, count, last_sync, errors)| PlatformSyncStatus {
                platform,
                connections_count: count,
                last_sync_at: last_sync,
                sync_errors_count: errors,
            })
            .collect())
    }

    // ========================================================================
    // OAuth Token Management (Story 96.2)
    // ========================================================================

    /// Find Airbnb connection by organization.
    pub async fn find_airbnb_connection_by_org(
        &self,
        org_id: Uuid,
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
            WHERE organization_id = $1 AND platform = 'airbnb'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
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
            WHERE platform = 'airbnb' AND external_property_id = $1 AND is_active = true
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(listing_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(conn)
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
            // PAP-158: cast the `platform` enum to text in RETURNING.
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
                RETURNING {}
                "#,
                Self::CONNECTION_COLUMNS
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

        // PAP-158: cast the `platform` enum to text in RETURNING.
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
            RETURNING {}
            "#,
            Self::CONNECTION_COLUMNS
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
            // PAP-158: cast the `platform` enum to text in RETURNING.
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
                RETURNING {}
                "#,
                Self::CONNECTION_COLUMNS
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

        // PAP-158: cast the `platform` enum to text in RETURNING.
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
            RETURNING {}
            "#,
            Self::CONNECTION_COLUMNS
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
        // PAP-158: cast the `platform` enum to text in RETURNING.
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
            RETURNING {}
            "#,
            Self::CONNECTION_COLUMNS
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
            WHERE platform = 'airbnb'
              AND is_active = true
              AND (encrypted_refresh_token IS NOT NULL OR refresh_token IS NOT NULL)
              AND token_expires_at IS NOT NULL
              AND token_expires_at <= $1
            ORDER BY token_expires_at ASC
            LIMIT 100
            "#,
        )
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
        let connection = sqlx::query_as::<_, RentalPlatformConnection>(
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
            WHERE organization_id = $1 AND platform = 'booking'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
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

        // PAP-158: cast the `platform` enum to text in RETURNING.
        let connection = sqlx::query_as::<_, RentalPlatformConnection>(sqlx::AssertSqlSafe(format!(
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
            RETURNING {}
            "#,
            Self::CONNECTION_COLUMNS
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

//! Rental booking CRUD, listing & check-in reminders (Story 18.2).

use super::RentalRepository;
use crate::models::rental::{
    block_reason, booking_status, BookingListQuery, BookingSummary, CheckInReminder, CreateBooking,
    RentalBooking, UpdateBooking, UpdateBookingStatus,
};
use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
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

    // ========================================================================
    // Bookings (Story 18.2)
    // ========================================================================

    /// Create booking.
    pub async fn create_booking(
        &self,
        org_id: Uuid,
        data: CreateBooking,
    ) -> Result<RentalBooking, SqlxError> {
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_bookings (
                organization_id, unit_id, platform, external_booking_id,
                guest_name, guest_email, guest_phone, guest_count,
                check_in, check_out, check_in_time, check_out_time,
                total_amount, currency, platform_fee, cleaning_fee,
                guest_notes, internal_notes, status
            )
            VALUES ($1, $2, $3::rental_platform, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19::rental_booking_status)
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
    ///
    /// Uses [`BOOKING_COLUMNS`] (enum columns cast to text) instead of a bare
    /// `SELECT *` to avoid the FORCE-RLS-masked 42804 enum decode gap (GH #1363).
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
    ///
    /// Uses [`BOOKING_COLUMNS`] (enum columns cast to text) instead of a bare
    /// `SELECT *` to avoid the FORCE-RLS-masked 42804 enum decode gap (GH #1363).
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
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {}
            FROM rental_bookings
            WHERE platform = $1::rental_platform AND external_booking_id = $2
            "#,
            Self::BOOKING_COLUMNS
        )))
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
        let booking = sqlx::query_as::<_, RentalBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_bookings SET
                status = $2::rental_booking_status,
                cancelled_at = CASE WHEN $2 = 'cancelled' THEN NOW() ELSE cancelled_at END,
                cancellation_reason = COALESCE($3, cancellation_reason),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $4
            RETURNING {}
            "#,
            Self::BOOKING_COLUMNS
        )))
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
            conditions.push(format!("b.platform = ${}", param_count));
        }
        if query.status.is_some() {
            param_count += 1;
            conditions.push(format!("b.status = ${}", param_count));
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
                b.id, b.unit_id, u.designation, COALESCE(bld.name, ''),
                b.platform::text, b.external_booking_id, b.guest_name, b.guest_count,
                b.check_in, b.check_out, b.total_amount, b.currency,
                b.status::text,
                (SELECT status FROM rental_guests WHERE booking_id = b.id AND is_primary = true LIMIT 1)
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
                b.id, u.designation, b.guest_name, b.check_in,
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
}

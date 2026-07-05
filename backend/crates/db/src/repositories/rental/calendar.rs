//! Rental calendar blocks, events & availability (Story 18.2).

use super::RentalRepository;
use super::CALENDAR_BLOCK_COLUMNS;
use crate::models::rental::{CalendarBlock, CalendarEvent, CreateCalendarBlock};
use chrono::NaiveDate;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // Calendar (Story 18.2)
    // ========================================================================

    /// Create calendar block.
    pub async fn create_calendar_block(
        &self,
        org_id: Uuid,
        data: CreateCalendarBlock,
    ) -> Result<CalendarBlock, SqlxError> {
        let block = sqlx::query_as::<_, CalendarBlock>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_calendar_blocks (organization_id, unit_id, block_start, block_end, reason, notes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING {CALENDAR_BLOCK_COLUMNS}
            "#
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
                let booking: Option<(String, String, String)> = sqlx::query_as(
                    r#"SELECT guest_name, platform::text, status FROM rental_bookings WHERE id = $1"#,
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
}

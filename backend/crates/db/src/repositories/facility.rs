//! Facility repository (Epic 3, Story 3.7).

use crate::models::facility::{
    CreateFacility, CreateFacilityBooking, Facility, FacilityBooking, FacilitySummary,
    UpdateFacility, UpdateFacilityBooking,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

// SECURITY/CORRECTNESS (#859): under sqlx 0.9 a Postgres enum column can no
// longer be decoded straight into a Rust `String` field — it fails at runtime
// with a 500. The SELECT readers in this module were fixed to cast the enum to
// text, but the `RETURNING *` clauses on the create/update/approve/reject/cancel
// mutations were missed: `*` returns the raw `facility_type` / `status` enum
// columns, so every facility/booking mutation that returns its row still 500s.
// These projections cast the enum column to text and are used in those
// `RETURNING` clauses (and mirror the column lists the SELECT readers use).

/// `RETURNING` projection for [`Facility`] mutations — casts the `facility_type`
/// enum to text so sqlx 0.9 decodes it into `Facility.facility_type: String`.
const FACILITY_RETURNING: &str = "id, building_id, name, facility_type::text AS facility_type, \
     description, location, capacity, is_bookable, requires_approval, max_booking_hours, \
     max_advance_days, min_advance_hours, available_from, available_to, available_days, rules, \
     hourly_fee, deposit_amount, is_active, photos, amenities, created_at, updated_at";

/// `RETURNING` projection for [`FacilityBooking`] mutations — casts the `status`
/// `booking_status` enum to text so sqlx 0.9 decodes it into
/// `FacilityBooking.status: String`.
const FACILITY_BOOKING_RETURNING: &str =
    "id, facility_id, user_id, unit_id, start_time, end_time, status::text AS status, purpose, \
     attendees, notes, approved_by, approved_at, rejected_by, rejected_at, rejection_reason, \
     cancelled_at, cancellation_reason, total_fee, deposit_paid, created_at, updated_at";

/// Repository for facility operations.
#[derive(Clone)]
pub struct FacilityRepository {
    pool: DbPool,
}

impl FacilityRepository {
    /// Create a new FacilityRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ==================== Facility Operations ====================

    /// Create a new facility.
    pub async fn create(&self, data: CreateFacility) -> Result<Facility, SqlxError> {
        let facility = sqlx::query_as::<_, Facility>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO facilities (
                building_id, name, facility_type, description, location,
                capacity, is_bookable, requires_approval, max_booking_hours,
                max_advance_days, min_advance_hours, available_from, available_to,
                available_days, rules, hourly_fee, deposit_amount
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING {FACILITY_RETURNING}
            "#
        )))
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.facility_type)
        .bind(&data.description)
        .bind(&data.location)
        .bind(data.capacity)
        .bind(data.is_bookable)
        .bind(data.requires_approval)
        .bind(data.max_booking_hours)
        .bind(data.max_advance_days)
        .bind(data.min_advance_hours)
        .bind(data.available_from)
        .bind(data.available_to)
        .bind(&data.available_days)
        .bind(&data.rules)
        .bind(data.hourly_fee)
        .bind(data.deposit_amount)
        .fetch_one(&self.pool)
        .await?;

        Ok(facility)
    }

    /// Find facility by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Facility>, SqlxError> {
        // `facility_type` is the `facility_type` Postgres enum; sqlx 0.9 refuses
        // to decode an enum column straight into a Rust `String`, so cast it to
        // text. (The `Facility` model keeps `facility_type: String`.) Columns are
        // listed explicitly because `SELECT *` cannot override one column's type.
        // See issue #859.
        let facility = sqlx::query_as::<_, Facility>(
            r#"SELECT id, building_id, name, facility_type::text AS facility_type,
                      description, location, capacity, is_bookable, requires_approval,
                      max_booking_hours, max_advance_days, min_advance_hours,
                      available_from, available_to, available_days, rules, hourly_fee,
                      deposit_amount, is_active, photos, amenities, created_at, updated_at
               FROM facilities WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(facility)
    }

    /// Find all facilities for a building.
    pub async fn find_by_building(
        &self,
        building_id: Uuid,
    ) -> Result<Vec<FacilitySummary>, SqlxError> {
        // `facility_type` is a Postgres enum; cast to text so sqlx 0.9 can decode
        // it into the `FacilitySummary.facility_type: String` field. See #859.
        let facilities = sqlx::query_as::<_, FacilitySummary>(
            r#"
            SELECT id, building_id, name, facility_type::text AS facility_type,
                   is_bookable, is_active
            FROM facilities
            WHERE building_id = $1
            ORDER BY name
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(facilities)
    }

    /// Find active bookable facilities for a building.
    pub async fn find_bookable(&self, building_id: Uuid) -> Result<Vec<Facility>, SqlxError> {
        // `facility_type` is a Postgres enum; cast to text for sqlx 0.9 decode
        // into `Facility.facility_type: String`. Explicit column list because
        // `SELECT *` cannot override one column's type. See #859.
        let facilities = sqlx::query_as::<_, Facility>(
            r#"
            SELECT id, building_id, name, facility_type::text AS facility_type,
                   description, location, capacity, is_bookable, requires_approval,
                   max_booking_hours, max_advance_days, min_advance_hours,
                   available_from, available_to, available_days, rules, hourly_fee,
                   deposit_amount, is_active, photos, amenities, created_at, updated_at
            FROM facilities
            WHERE building_id = $1 AND is_active AND is_bookable
            ORDER BY name
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(facilities)
    }

    /// Update a facility.
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateFacility,
    ) -> Result<Option<Facility>, SqlxError> {
        let facility = sqlx::query_as::<_, Facility>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE facilities
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                location = COALESCE($4, location),
                capacity = COALESCE($5, capacity),
                is_bookable = COALESCE($6, is_bookable),
                requires_approval = COALESCE($7, requires_approval),
                max_booking_hours = COALESCE($8, max_booking_hours),
                max_advance_days = COALESCE($9, max_advance_days),
                min_advance_hours = COALESCE($10, min_advance_hours),
                available_from = COALESCE($11, available_from),
                available_to = COALESCE($12, available_to),
                available_days = COALESCE($13, available_days),
                rules = COALESCE($14, rules),
                hourly_fee = COALESCE($15, hourly_fee),
                deposit_amount = COALESCE($16, deposit_amount),
                is_active = COALESCE($17, is_active),
                photos = COALESCE($18, photos),
                amenities = COALESCE($19, amenities),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {FACILITY_RETURNING}
            "#
        )))
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.location)
        .bind(data.capacity)
        .bind(data.is_bookable)
        .bind(data.requires_approval)
        .bind(data.max_booking_hours)
        .bind(data.max_advance_days)
        .bind(data.min_advance_hours)
        .bind(data.available_from)
        .bind(data.available_to)
        .bind(&data.available_days)
        .bind(&data.rules)
        .bind(data.hourly_fee)
        .bind(data.deposit_amount)
        .bind(data.is_active)
        .bind(&data.photos)
        .bind(&data.amenities)
        .fetch_optional(&self.pool)
        .await?;

        Ok(facility)
    }

    /// Delete a facility.
    pub async fn delete(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM facilities WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Booking Operations ====================

    /// Create a new booking.
    pub async fn create_booking(
        &self,
        user_id: Uuid,
        data: CreateFacilityBooking,
    ) -> Result<FacilityBooking, SqlxError> {
        // Get facility to check if requires approval
        let facility = self.find_by_id(data.facility_id).await?;
        let status = match facility {
            Some(f) if f.requires_approval => "pending",
            _ => "approved",
        };

        let booking = sqlx::query_as::<_, FacilityBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO facility_bookings (
                facility_id, user_id, unit_id, start_time, end_time,
                status, purpose, attendees, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING {FACILITY_BOOKING_RETURNING}
            "#
        )))
        .bind(data.facility_id)
        .bind(user_id)
        .bind(data.unit_id)
        .bind(data.start_time)
        .bind(data.end_time)
        .bind(status)
        .bind(&data.purpose)
        .bind(data.attendees)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Find booking by ID.
    pub async fn find_booking_by_id(&self, id: Uuid) -> Result<Option<FacilityBooking>, SqlxError> {
        // `status` is the `booking_status` Postgres enum; sqlx 0.9 refuses to
        // decode an enum column straight into a Rust `String`, so cast it to
        // text. (The `FacilityBooking` model keeps `status: String`.) Columns
        // are listed explicitly because `SELECT *` cannot override one column's
        // type. See the sqlx-0.9 enum-decode tracking issue for the rest.
        let booking = sqlx::query_as::<_, FacilityBooking>(
            r#"SELECT id, facility_id, user_id, unit_id, start_time, end_time,
                      status::text AS status, purpose, attendees, notes,
                      approved_by, approved_at, rejected_by, rejected_at,
                      rejection_reason, cancelled_at, cancellation_reason,
                      total_fee, deposit_paid, created_at, updated_at
               FROM facility_bookings WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Find bookings for a facility in a time range.
    pub async fn find_bookings_by_facility(
        &self,
        facility_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<FacilityBooking>, SqlxError> {
        // `status` is the `booking_status` Postgres enum; cast to text so sqlx
        // 0.9 can decode it into `FacilityBooking.status: String`. See #859.
        let bookings = sqlx::query_as::<_, FacilityBooking>(
            r#"
            SELECT id, facility_id, user_id, unit_id, start_time, end_time,
                   status::text AS status, purpose, attendees, notes,
                   approved_by, approved_at, rejected_by, rejected_at,
                   rejection_reason, cancelled_at, cancellation_reason,
                   total_fee, deposit_paid, created_at, updated_at
            FROM facility_bookings
            WHERE facility_id = $1
              AND start_time < $3
              AND end_time > $2
              AND status IN ('pending', 'approved')
            ORDER BY start_time
            "#,
        )
        .bind(facility_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(bookings)
    }

    /// Find bookings for a user.
    pub async fn find_bookings_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FacilityBooking>, SqlxError> {
        // `status` is the `booking_status` Postgres enum; cast to text for sqlx
        // 0.9 decode into `FacilityBooking.status: String`. See #859.
        let bookings = sqlx::query_as::<_, FacilityBooking>(
            r#"
            SELECT id, facility_id, user_id, unit_id, start_time, end_time,
                   status::text AS status, purpose, attendees, notes,
                   approved_by, approved_at, rejected_by, rejected_at,
                   rejection_reason, cancelled_at, cancellation_reason,
                   total_fee, deposit_paid, created_at, updated_at
            FROM facility_bookings
            WHERE user_id = $1
            ORDER BY start_time DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(bookings)
    }

    /// Find pending bookings for approval.
    pub async fn find_pending_bookings(
        &self,
        building_id: Uuid,
    ) -> Result<Vec<FacilityBooking>, SqlxError> {
        // `status` is the `booking_status` Postgres enum; cast to text for sqlx
        // 0.9 decode into `FacilityBooking.status: String`. See #859.
        let bookings = sqlx::query_as::<_, FacilityBooking>(
            r#"
            SELECT fb.id, fb.facility_id, fb.user_id, fb.unit_id, fb.start_time,
                   fb.end_time, fb.status::text AS status, fb.purpose, fb.attendees,
                   fb.notes, fb.approved_by, fb.approved_at, fb.rejected_by,
                   fb.rejected_at, fb.rejection_reason, fb.cancelled_at,
                   fb.cancellation_reason, fb.total_fee, fb.deposit_paid,
                   fb.created_at, fb.updated_at
            FROM facility_bookings fb
            INNER JOIN facilities f ON f.id = fb.facility_id
            WHERE f.building_id = $1 AND fb.status = 'pending'
            ORDER BY fb.created_at
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(bookings)
    }

    /// Update a booking.
    pub async fn update_booking(
        &self,
        id: Uuid,
        data: UpdateFacilityBooking,
    ) -> Result<Option<FacilityBooking>, SqlxError> {
        let booking = sqlx::query_as::<_, FacilityBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE facility_bookings
            SET
                start_time = COALESCE($2, start_time),
                end_time = COALESCE($3, end_time),
                purpose = COALESCE($4, purpose),
                attendees = COALESCE($5, attendees),
                notes = COALESCE($6, notes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {FACILITY_BOOKING_RETURNING}
            "#
        )))
        .bind(id)
        .bind(data.start_time)
        .bind(data.end_time)
        .bind(&data.purpose)
        .bind(data.attendees)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Approve a booking.
    pub async fn approve_booking(
        &self,
        id: Uuid,
        approved_by: Uuid,
    ) -> Result<Option<FacilityBooking>, SqlxError> {
        let booking = sqlx::query_as::<_, FacilityBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE facility_bookings
            SET status = 'approved', approved_by = $2, approved_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING {FACILITY_BOOKING_RETURNING}
            "#
        )))
        .bind(id)
        .bind(approved_by)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Reject a booking.
    pub async fn reject_booking(
        &self,
        id: Uuid,
        rejected_by: Uuid,
        reason: &str,
    ) -> Result<Option<FacilityBooking>, SqlxError> {
        let booking = sqlx::query_as::<_, FacilityBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE facility_bookings
            SET status = 'rejected', rejected_by = $2, rejected_at = NOW(),
                rejection_reason = $3, updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING {FACILITY_BOOKING_RETURNING}
            "#
        )))
        .bind(id)
        .bind(rejected_by)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Cancel a booking.
    pub async fn cancel_booking(
        &self,
        id: Uuid,
        reason: Option<&str>,
    ) -> Result<Option<FacilityBooking>, SqlxError> {
        let booking = sqlx::query_as::<_, FacilityBooking>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE facility_bookings
            SET status = 'cancelled', cancelled_at = NOW(),
                cancellation_reason = $2, updated_at = NOW()
            WHERE id = $1 AND status IN ('pending', 'approved')
            RETURNING {FACILITY_BOOKING_RETURNING}
            "#
        )))
        .bind(id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await?;

        Ok(booking)
    }

    /// Check facility availability for a time slot.
    pub async fn check_availability(
        &self,
        facility_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        exclude_booking_id: Option<Uuid>,
    ) -> Result<bool, SqlxError> {
        let conflict: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM facility_bookings
                WHERE facility_id = $1
                  AND status IN ('pending', 'approved')
                  AND ($4::UUID IS NULL OR id != $4)
                  AND start_time < $3
                  AND end_time > $2
            )
            "#,
        )
        .bind(facility_id)
        .bind(start_time)
        .bind(end_time)
        .bind(exclude_booking_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(!conflict.0)
    }

    /// Delete a booking.
    pub async fn delete_booking(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM facility_bookings WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

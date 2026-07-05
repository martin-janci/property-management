//! Rental guest CRUD, registration & booking-with-guests (Story 18.3).

use super::RentalRepository;
use super::GUEST_COLUMNS;
use crate::models::rental::{
    guest_status, BookingWithGuests, CreateGuest, RentalGuest, UpdateGuest,
};
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // Guests (Story 18.3)
    // ========================================================================

    /// Create guest.
    pub async fn create_guest(
        &self,
        org_id: Uuid,
        data: CreateGuest,
    ) -> Result<RentalGuest, SqlxError> {
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO rental_guests (
                organization_id, booking_id, first_name, last_name,
                date_of_birth, nationality, id_type, id_number,
                id_issuing_country, id_expiry_date, email, phone,
                address_street, address_city, address_postal_code, address_country,
                is_primary, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18::guest_registration_status)
            RETURNING {GUEST_COLUMNS}
            "#
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
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            "SELECT {GUEST_COLUMNS} FROM rental_guests WHERE id = $1"
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
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            "SELECT {GUEST_COLUMNS} FROM rental_guests WHERE id = $1 AND organization_id = $2"
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
            RETURNING {GUEST_COLUMNS}
            "#
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
            RETURNING {GUEST_COLUMNS}
            "#
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
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_guests SET
                status = $2::guest_registration_status,
                registered_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING {GUEST_COLUMNS}
            "#
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
        let guest = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE rental_guests SET
                status = $2::guest_registration_status,
                registered_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $3
            RETURNING {GUEST_COLUMNS}
            "#
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
        let guests = sqlx::query_as::<_, RentalGuest>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {GUEST_COLUMNS} FROM rental_guests
            WHERE booking_id = $1
            ORDER BY is_primary DESC, created_at
            "#
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
}

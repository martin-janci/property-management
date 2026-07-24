//! Registry repository for Epic 57.
//!
//! Handles all database operations for pet and vehicle registrations,
//! parking spots, and building registry rules.

use crate::models::{
    registry_status, BuildingRegistryRules, CreateParkingSpot, CreatePetRegistration,
    CreateVehicleRegistration, ParkingSpot, ParkingSpotQuery, ParkingSpotWithDetails,
    PetRegistration, PetRegistrationQuery, PetRegistrationSummary, PetRegistrationWithDetails,
    RegistryStatistics, ReviewRegistration, UpdateParkingSpot, UpdatePetRegistration,
    UpdateRegistryRules, UpdateVehicleRegistration, VehicleRegistration, VehicleRegistrationQuery,
    VehicleRegistrationSummary, VehicleRegistrationWithDetails,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Repository for registry-related database operations.
#[derive(Clone)]
pub struct RegistryRepository {
    pool: PgPool,
}

impl RegistryRepository {
    /// Creates a new registry repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Pet Registration Operations
    // ========================================================================

    /// Creates a new pet registration.
    pub async fn create_pet_registration(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
        data: CreatePetRegistration,
    ) -> Result<PetRegistration, sqlx::Error> {
        sqlx::query_as::<_, PetRegistration>(
            r#"
            INSERT INTO pet_registrations (
                tenant_id, unit_id, owner_id, pet_name, pet_type, breed,
                pet_size, weight_kg, age_years, color, microchip_id,
                vaccination_date, vaccination_document_id, special_needs, notes, status
            )
            -- pet_type / pet_size / status are ENUM columns; the String binds arrive
            -- as text, so cast the placeholders explicitly (text -> enum is not an
            -- implicit assignment cast in Postgres).
            VALUES ($1, $2, $3, $4, $5::pet_type, $6, $7::pet_size, $8, $9, $10, $11, $12, $13, $14, $15, $16::registry_status)
            RETURNING
                id, tenant_id, unit_id, owner_id, pet_name,
                pet_type::text AS pet_type, breed, pet_size::text AS pet_size,
                weight_kg, age_years, color, microchip_id, vaccination_date,
                vaccination_document_id, special_needs, status::text AS status,
                reviewed_by, reviewed_at, rejection_reason, notes, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(data.unit_id)
        .bind(owner_id)
        .bind(&data.pet_name)
        .bind(&data.pet_type)
        .bind(&data.breed)
        .bind(&data.pet_size)
        .bind(data.weight_kg)
        .bind(data.age_years)
        .bind(&data.color)
        .bind(&data.microchip_id)
        .bind(data.vaccination_date)
        .bind(data.vaccination_document_id)
        .bind(&data.special_needs)
        .bind(&data.notes)
        .bind(registry_status::PENDING)
        .fetch_one(&self.pool)
        .await
    }

    /// Gets a pet registration by ID.
    pub async fn get_pet_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<PetRegistration>, sqlx::Error> {
        // `pet_type` / `pet_size` / `status` are Postgres ENUMs; the model decodes
        // them as `String`, so cast them to text explicitly. A bare `SELECT *` only
        // ever succeeded because FORCE RLS normally returns zero rows here — see
        // PAP-143 / PAP-136 "deny-all-masked decode" notes.
        sqlx::query_as::<_, PetRegistration>(
            r#"
            SELECT id, tenant_id, unit_id, owner_id, pet_name,
                   pet_type::text AS pet_type, breed, pet_size::text AS pet_size,
                   weight_kg, age_years, color, microchip_id, vaccination_date,
                   vaccination_document_id, special_needs, status::text AS status,
                   reviewed_by, reviewed_at, rejection_reason, notes, created_at, updated_at
            FROM pet_registrations
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Gets a pet registration with all details.
    pub async fn get_pet_registration_with_details(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<PetRegistrationWithDetails>, sqlx::Error> {
        let registration = match self.get_pet_registration(tenant_id, id).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        // `units` has no `unit_number` column — the display value is `designation`.
        // `buildings.name` is nullable, so decode as Option<String> to avoid ColumnDecode errors.
        let unit_info: Option<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT u.designation AS unit_number, b.name as building_name
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE u.id = $1
            "#,
        )
        .bind(registration.unit_id)
        .fetch_optional(&self.pool)
        .await?;

        let owner_name: Option<String> =
            sqlx::query_scalar("SELECT COALESCE(name, email) FROM users WHERE id = $1")
                .bind(registration.owner_id)
                .fetch_optional(&self.pool)
                .await?;

        let reviewed_by_name = if let Some(reviewed_by) = registration.reviewed_by {
            sqlx::query_scalar::<_, String>("SELECT COALESCE(name, email) FROM users WHERE id = $1")
                .bind(reviewed_by)
                .fetch_optional(&self.pool)
                .await?
        } else {
            None
        };

        Ok(Some(PetRegistrationWithDetails {
            registration,
            unit_number: unit_info.as_ref().map(|(u, _)| u.clone()),
            building_name: unit_info.and_then(|(_, b)| b),
            owner_name,
            reviewed_by_name,
        }))
    }

    /// Lists pet registrations with filtering.
    pub async fn list_pet_registrations(
        &self,
        tenant_id: Uuid,
        query: PetRegistrationQuery,
    ) -> Result<(Vec<PetRegistrationSummary>, i64), sqlx::Error> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM pet_registrations pr
            JOIN units u ON u.id = pr.unit_id
            WHERE pr.tenant_id = $1
                AND ($2::uuid IS NULL OR u.building_id = $2)
                AND ($3::uuid IS NULL OR pr.unit_id = $3)
                AND ($4::uuid IS NULL OR pr.owner_id = $4)
                AND ($5::text IS NULL OR pr.status::text = $5)
                AND ($6::text IS NULL OR pr.pet_type::text = $6)
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.owner_id)
        .bind(&query.status)
        .bind(&query.pet_type)
        .fetch_one(&self.pool)
        .await?;

        let registrations = sqlx::query_as::<_, PetRegistrationSummary>(
            r#"
            SELECT pr.id, pr.unit_id, u.designation AS unit_number,
                   pr.owner_id, COALESCE(usr.name, usr.email) as owner_name,
                   pr.pet_name, pr.pet_type::text AS pet_type, pr.breed,
                   pr.pet_size::text AS pet_size, pr.status::text AS status, pr.created_at
            FROM pet_registrations pr
            JOIN units u ON u.id = pr.unit_id
            LEFT JOIN users usr ON usr.id = pr.owner_id
            WHERE pr.tenant_id = $1
                AND ($2::uuid IS NULL OR u.building_id = $2)
                AND ($3::uuid IS NULL OR pr.unit_id = $3)
                AND ($4::uuid IS NULL OR pr.owner_id = $4)
                AND ($5::text IS NULL OR pr.status::text = $5)
                AND ($6::text IS NULL OR pr.pet_type::text = $6)
            ORDER BY pr.created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.owner_id)
        .bind(&query.status)
        .bind(&query.pet_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((registrations, total))
    }

    /// Updates a pet registration.
    pub async fn update_pet_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        data: UpdatePetRegistration,
    ) -> Result<Option<PetRegistration>, sqlx::Error> {
        sqlx::query_as::<_, PetRegistration>(
            r#"
            UPDATE pet_registrations SET
                pet_name = COALESCE($3, pet_name),
                breed = COALESCE($4, breed),
                pet_size = COALESCE($5::pet_size, pet_size),
                weight_kg = COALESCE($6, weight_kg),
                age_years = COALESCE($7, age_years),
                color = COALESCE($8, color),
                microchip_id = COALESCE($9, microchip_id),
                vaccination_date = COALESCE($10, vaccination_date),
                vaccination_document_id = COALESCE($11, vaccination_document_id),
                special_needs = COALESCE($12, special_needs),
                notes = COALESCE($13, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING
                id, tenant_id, unit_id, owner_id, pet_name,
                pet_type::text AS pet_type, breed, pet_size::text AS pet_size,
                weight_kg, age_years, color, microchip_id, vaccination_date,
                vaccination_document_id, special_needs, status::text AS status,
                reviewed_by, reviewed_at, rejection_reason, notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&data.pet_name)
        .bind(&data.breed)
        .bind(&data.pet_size)
        .bind(data.weight_kg)
        .bind(data.age_years)
        .bind(&data.color)
        .bind(&data.microchip_id)
        .bind(data.vaccination_date)
        .bind(data.vaccination_document_id)
        .bind(&data.special_needs)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Reviews (approves/rejects) a pet registration.
    pub async fn review_pet_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        reviewer_id: Uuid,
        data: ReviewRegistration,
    ) -> Result<Option<PetRegistration>, sqlx::Error> {
        let status = if data.approve {
            registry_status::APPROVED
        } else {
            registry_status::REJECTED
        };

        sqlx::query_as::<_, PetRegistration>(
            r#"
            UPDATE pet_registrations SET
                status = $3::registry_status,
                reviewed_by = $4,
                reviewed_at = $5,
                rejection_reason = $6,
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING
                id, tenant_id, unit_id, owner_id, pet_name,
                pet_type::text AS pet_type, breed, pet_size::text AS pet_size,
                weight_kg, age_years, color, microchip_id, vaccination_date,
                vaccination_document_id, special_needs, status::text AS status,
                reviewed_by, reviewed_at, rejection_reason, notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(status)
        .bind(reviewer_id)
        .bind(Utc::now())
        .bind(&data.rejection_reason)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a pet registration.
    pub async fn delete_pet_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM pet_registrations WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Vehicle Registration Operations
    // ========================================================================

    /// Creates a new vehicle registration.
    pub async fn create_vehicle_registration(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
        data: CreateVehicleRegistration,
    ) -> Result<VehicleRegistration, sqlx::Error> {
        sqlx::query_as::<_, VehicleRegistration>(
            r#"
            INSERT INTO vehicle_registrations (
                tenant_id, unit_id, owner_id, vehicle_type, make, model, year,
                color, license_plate, registration_document_id, insurance_document_id, notes, status
            )
            -- vehicle_type / status are ENUM columns; cast the String-bound
            -- placeholders explicitly (text -> enum is not an implicit cast).
            VALUES ($1, $2, $3, $4::vehicle_type, $5, $6, $7, $8, $9, $10, $11, $12, $13::registry_status)
            RETURNING
                id, tenant_id, unit_id, owner_id,
                vehicle_type::text AS vehicle_type, make, model, year, color,
                license_plate, registration_document_id, insurance_document_id,
                parking_spot_id, status::text AS status, reviewed_by, reviewed_at,
                rejection_reason, notes, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(data.unit_id)
        .bind(owner_id)
        .bind(&data.vehicle_type)
        .bind(&data.make)
        .bind(&data.model)
        .bind(data.year)
        .bind(&data.color)
        .bind(&data.license_plate)
        .bind(data.registration_document_id)
        .bind(data.insurance_document_id)
        .bind(&data.notes)
        .bind(registry_status::PENDING)
        .fetch_one(&self.pool)
        .await
    }

    /// Gets a vehicle registration by ID.
    pub async fn get_vehicle_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<VehicleRegistration>, sqlx::Error> {
        // `vehicle_type` / `status` are Postgres ENUMs; the model decodes them as
        // `String`, so cast them to text explicitly. A bare `SELECT *` only ever
        // succeeded because FORCE RLS normally returns zero rows here — see PAP-143
        // / PAP-136 "deny-all-masked decode" notes.
        sqlx::query_as::<_, VehicleRegistration>(
            r#"
            SELECT id, tenant_id, unit_id, owner_id,
                   vehicle_type::text AS vehicle_type, make, model, year, color,
                   license_plate, registration_document_id, insurance_document_id,
                   parking_spot_id, status::text AS status, reviewed_by, reviewed_at,
                   rejection_reason, notes, created_at, updated_at
            FROM vehicle_registrations
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Gets a vehicle registration with all details.
    pub async fn get_vehicle_registration_with_details(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<VehicleRegistrationWithDetails>, sqlx::Error> {
        let registration = match self.get_vehicle_registration(tenant_id, id).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        // `units` has no `unit_number` column — the display value is `designation`.
        let unit_info: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT u.designation AS unit_number, b.name as building_name
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE u.id = $1
            "#,
        )
        .bind(registration.unit_id)
        .fetch_optional(&self.pool)
        .await?;

        let owner_name: Option<String> =
            sqlx::query_scalar("SELECT COALESCE(name, email) FROM users WHERE id = $1")
                .bind(registration.owner_id)
                .fetch_optional(&self.pool)
                .await?;

        let reviewed_by_name = if let Some(reviewed_by) = registration.reviewed_by {
            sqlx::query_scalar::<_, String>("SELECT COALESCE(name, email) FROM users WHERE id = $1")
                .bind(reviewed_by)
                .fetch_optional(&self.pool)
                .await?
        } else {
            None
        };

        let parking_spot_number = if let Some(spot_id) = registration.parking_spot_id {
            // Defense-in-depth: tenant-scope the secondary lookup like the rest of
            // registry.rs, so a cross-tenant parking_spot_id can never leak a spot
            // number even if FORCE RLS is bypassed (e.g. superuser pool). PAP-143.
            sqlx::query_scalar::<_, String>(
                "SELECT spot_number FROM parking_spots WHERE id = $1 AND tenant_id = $2",
            )
            .bind(spot_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        Ok(Some(VehicleRegistrationWithDetails {
            registration,
            unit_number: unit_info.as_ref().map(|(u, _)| u.clone()),
            building_name: unit_info.map(|(_, b)| b),
            owner_name,
            reviewed_by_name,
            parking_spot_number,
        }))
    }

    /// Lists vehicle registrations with filtering.
    pub async fn list_vehicle_registrations(
        &self,
        tenant_id: Uuid,
        query: VehicleRegistrationQuery,
    ) -> Result<(Vec<VehicleRegistrationSummary>, i64), sqlx::Error> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM vehicle_registrations vr
            JOIN units u ON u.id = vr.unit_id
            WHERE vr.tenant_id = $1
                AND ($2::uuid IS NULL OR u.building_id = $2)
                AND ($3::uuid IS NULL OR vr.unit_id = $3)
                AND ($4::uuid IS NULL OR vr.owner_id = $4)
                AND ($5::text IS NULL OR vr.status::text = $5)
                AND ($6::text IS NULL OR vr.vehicle_type::text = $6)
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.owner_id)
        .bind(&query.status)
        .bind(&query.vehicle_type)
        .fetch_one(&self.pool)
        .await?;

        let registrations = sqlx::query_as::<_, VehicleRegistrationSummary>(
            r#"
            SELECT vr.id, vr.unit_id, u.designation AS unit_number,
                   vr.owner_id, COALESCE(usr.name, usr.email) as owner_name,
                   vr.vehicle_type::text AS vehicle_type, vr.make, vr.model,
                   vr.license_plate, vr.status::text AS status,
                   ps.spot_number as parking_spot_number, vr.created_at
            FROM vehicle_registrations vr
            JOIN units u ON u.id = vr.unit_id
            LEFT JOIN users usr ON usr.id = vr.owner_id
            -- Tenant-scope the spot join: parking_spot_id has no FK, so a stale
            -- cross-tenant id must not leak a foreign spot_number here (PAP-143 DiD).
            LEFT JOIN parking_spots ps ON ps.id = vr.parking_spot_id AND ps.tenant_id = vr.tenant_id
            WHERE vr.tenant_id = $1
                AND ($2::uuid IS NULL OR u.building_id = $2)
                AND ($3::uuid IS NULL OR vr.unit_id = $3)
                AND ($4::uuid IS NULL OR vr.owner_id = $4)
                AND ($5::text IS NULL OR vr.status::text = $5)
                AND ($6::text IS NULL OR vr.vehicle_type::text = $6)
            ORDER BY vr.created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.owner_id)
        .bind(&query.status)
        .bind(&query.vehicle_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((registrations, total))
    }

    /// Updates a vehicle registration.
    pub async fn update_vehicle_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        data: UpdateVehicleRegistration,
    ) -> Result<Option<VehicleRegistration>, sqlx::Error> {
        sqlx::query_as::<_, VehicleRegistration>(
            r#"
            UPDATE vehicle_registrations SET
                make = COALESCE($3, make),
                model = COALESCE($4, model),
                year = COALESCE($5, year),
                color = COALESCE($6, color),
                license_plate = COALESCE($7, license_plate),
                registration_document_id = COALESCE($8, registration_document_id),
                insurance_document_id = COALESCE($9, insurance_document_id),
                parking_spot_id = COALESCE($10, parking_spot_id),
                notes = COALESCE($11, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING
                id, tenant_id, unit_id, owner_id,
                vehicle_type::text AS vehicle_type, make, model, year, color,
                license_plate, registration_document_id, insurance_document_id,
                parking_spot_id, status::text AS status, reviewed_by, reviewed_at,
                rejection_reason, notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&data.make)
        .bind(&data.model)
        .bind(data.year)
        .bind(&data.color)
        .bind(&data.license_plate)
        .bind(data.registration_document_id)
        .bind(data.insurance_document_id)
        .bind(data.parking_spot_id)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Reviews (approves/rejects) a vehicle registration.
    pub async fn review_vehicle_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        reviewer_id: Uuid,
        data: ReviewRegistration,
    ) -> Result<Option<VehicleRegistration>, sqlx::Error> {
        let status = if data.approve {
            registry_status::APPROVED
        } else {
            registry_status::REJECTED
        };

        sqlx::query_as::<_, VehicleRegistration>(
            r#"
            UPDATE vehicle_registrations SET
                status = $3::registry_status,
                reviewed_by = $4,
                reviewed_at = $5,
                rejection_reason = $6,
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING
                id, tenant_id, unit_id, owner_id,
                vehicle_type::text AS vehicle_type, make, model, year, color,
                license_plate, registration_document_id, insurance_document_id,
                parking_spot_id, status::text AS status, reviewed_by, reviewed_at,
                rejection_reason, notes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(status)
        .bind(reviewer_id)
        .bind(Utc::now())
        .bind(&data.rejection_reason)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a vehicle registration.
    pub async fn delete_vehicle_registration(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM vehicle_registrations WHERE id = $1 AND tenant_id = $2")
                .bind(id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Parking Spot Operations
    // ========================================================================

    /// Creates a new parking spot.
    pub async fn create_parking_spot(
        &self,
        tenant_id: Uuid,
        data: CreateParkingSpot,
    ) -> Result<ParkingSpot, sqlx::Error> {
        sqlx::query_as::<_, ParkingSpot>(
            r#"
            INSERT INTO parking_spots (
                tenant_id, building_id, spot_number, spot_type, floor_level,
                is_covered, is_reserved, assigned_to_unit_id, monthly_fee, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(data.building_id)
        .bind(&data.spot_number)
        .bind(data.spot_type.as_deref().unwrap_or("standard"))
        .bind(&data.floor_level)
        .bind(data.is_covered.unwrap_or(false))
        .bind(data.is_reserved.unwrap_or(false))
        .bind(data.assigned_to_unit_id)
        .bind(data.monthly_fee)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Gets a parking spot by ID.
    pub async fn get_parking_spot(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ParkingSpot>, sqlx::Error> {
        sqlx::query_as::<_, ParkingSpot>(
            "SELECT * FROM parking_spots WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Lists parking spots with filtering.
    pub async fn list_parking_spots(
        &self,
        tenant_id: Uuid,
        query: ParkingSpotQuery,
    ) -> Result<(Vec<ParkingSpotWithDetails>, i64), sqlx::Error> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        // Note: is_available filter is handled directly in the SQL WHERE clause

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM parking_spots
            WHERE tenant_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::bool IS NULL OR is_covered = $3)
                AND ($4::bool IS NULL OR (
                    (CASE WHEN $4 THEN assigned_to_unit_id IS NULL ELSE assigned_to_unit_id IS NOT NULL END)
                ))
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.is_covered)
        .bind(query.is_available)
        .fetch_one(&self.pool)
        .await?;

        let spots = sqlx::query_as::<_, ParkingSpotWithDetails>(
            r#"
            SELECT ps.id, ps.tenant_id, ps.building_id, ps.spot_number, ps.spot_type,
                   ps.floor_level, ps.is_covered, ps.is_reserved, ps.assigned_to_unit_id,
                   ps.monthly_fee, ps.notes, ps.created_at, ps.updated_at,
                   u.designation as assigned_unit_number, b.name as building_name
            FROM parking_spots ps
            LEFT JOIN units u ON u.id = ps.assigned_to_unit_id
            JOIN buildings b ON b.id = ps.building_id
            WHERE ps.tenant_id = $1
                AND ($2::uuid IS NULL OR ps.building_id = $2)
                AND ($3::bool IS NULL OR ps.is_covered = $3)
                AND ($4::bool IS NULL OR (
                    (CASE WHEN $4 THEN ps.assigned_to_unit_id IS NULL ELSE ps.assigned_to_unit_id IS NOT NULL END)
                ))
            ORDER BY ps.spot_number ASC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.is_covered)
        .bind(query.is_available)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((spots, total))
    }

    /// Updates a parking spot.
    pub async fn update_parking_spot(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        data: UpdateParkingSpot,
    ) -> Result<Option<ParkingSpot>, sqlx::Error> {
        sqlx::query_as::<_, ParkingSpot>(
            r#"
            UPDATE parking_spots SET
                spot_number = COALESCE($3, spot_number),
                spot_type = COALESCE($4, spot_type),
                floor_level = COALESCE($5, floor_level),
                is_covered = COALESCE($6, is_covered),
                is_reserved = COALESCE($7, is_reserved),
                assigned_to_unit_id = COALESCE($8, assigned_to_unit_id),
                monthly_fee = COALESCE($9, monthly_fee),
                notes = COALESCE($10, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&data.spot_number)
        .bind(&data.spot_type)
        .bind(&data.floor_level)
        .bind(data.is_covered)
        .bind(data.is_reserved)
        .bind(data.assigned_to_unit_id)
        .bind(data.monthly_fee)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a parking spot.
    pub async fn delete_parking_spot(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM parking_spots WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Building Registry Rules Operations
    // ========================================================================

    /// Gets registry rules for a building.
    pub async fn get_registry_rules(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
    ) -> Result<Option<BuildingRegistryRules>, sqlx::Error> {
        sqlx::query_as::<_, BuildingRegistryRules>(
            r#"
            SELECT
                id, tenant_id, building_id, pets_allowed, pets_require_approval,
                max_pets_per_unit, allowed_pet_types::text[] AS allowed_pet_types,
                banned_pet_breeds, max_pet_weight, vehicles_require_approval,
                max_vehicles_per_unit, notes, created_at, updated_at
            FROM building_registry_rules
            WHERE building_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(building_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Creates or updates registry rules for a building.
    pub async fn upsert_registry_rules(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
        data: UpdateRegistryRules,
    ) -> Result<BuildingRegistryRules, sqlx::Error> {
        sqlx::query_as::<_, BuildingRegistryRules>(
            r#"
            INSERT INTO building_registry_rules (
                tenant_id, building_id, pets_allowed, pets_require_approval,
                max_pets_per_unit, allowed_pet_types, banned_pet_breeds,
                max_pet_weight, vehicles_require_approval, max_vehicles_per_unit, notes
            )
            -- allowed_pet_types is pet_type[] enum; cast the text[] bind explicitly.
            -- pets_allowed / pets_require_approval / vehicles_require_approval are
            -- NOT NULL columns bound from Option<bool>; COALESCE a None to the
            -- column's schema DEFAULT (00067_create_building_registries.sql:
            -- pets_allowed/pets_require_approval default TRUE, vehicles_require_approval FALSE).
            VALUES (
                $1, $2,
                COALESCE($3, TRUE),
                COALESCE($4, TRUE),
                $5, $6::text[]::pet_type[], $7, $8,
                COALESCE($9, FALSE),
                $10, $11
            )
            ON CONFLICT (tenant_id, building_id) DO UPDATE SET
                pets_allowed = COALESCE($3, building_registry_rules.pets_allowed),
                pets_require_approval = COALESCE($4, building_registry_rules.pets_require_approval),
                max_pets_per_unit = COALESCE($5, building_registry_rules.max_pets_per_unit),
                allowed_pet_types = COALESCE($6::text[]::pet_type[], building_registry_rules.allowed_pet_types),
                banned_pet_breeds = COALESCE($7, building_registry_rules.banned_pet_breeds),
                max_pet_weight = COALESCE($8, building_registry_rules.max_pet_weight),
                vehicles_require_approval = COALESCE($9, building_registry_rules.vehicles_require_approval),
                max_vehicles_per_unit = COALESCE($10, building_registry_rules.max_vehicles_per_unit),
                notes = COALESCE($11, building_registry_rules.notes),
                updated_at = NOW()
            RETURNING
                id, tenant_id, building_id, pets_allowed, pets_require_approval,
                max_pets_per_unit, allowed_pet_types::text[] AS allowed_pet_types,
                banned_pet_breeds, max_pet_weight, vehicles_require_approval,
                max_vehicles_per_unit, notes, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .bind(data.pets_allowed)
        .bind(data.pets_require_approval)
        .bind(data.max_pets_per_unit)
        .bind(&data.allowed_pet_types)
        .bind(&data.banned_pet_breeds)
        .bind(data.max_pet_weight)
        .bind(data.vehicles_require_approval)
        .bind(data.max_vehicles_per_unit)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Gets registry statistics for a building.
    pub async fn get_statistics(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
    ) -> Result<RegistryStatistics, sqlx::Error> {
        let pet_stats: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE pr.status = 'pending') as pending,
                COUNT(*) FILTER (WHERE pr.status = 'approved') as approved
            FROM pet_registrations pr
            JOIN units u ON u.id = pr.unit_id
            WHERE pr.tenant_id = $1 AND u.building_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        let vehicle_stats: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE vr.status = 'pending') as pending,
                COUNT(*) FILTER (WHERE vr.status = 'approved') as approved
            FROM vehicle_registrations vr
            JOIN units u ON u.id = vr.unit_id
            WHERE vr.tenant_id = $1 AND u.building_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        let parking_stats: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE assigned_to_unit_id IS NULL) as available
            FROM parking_spots
            WHERE tenant_id = $1 AND building_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(RegistryStatistics {
            total_pets: pet_stats.0,
            pending_pets: pet_stats.1,
            approved_pets: pet_stats.2,
            total_vehicles: vehicle_stats.0,
            pending_vehicles: vehicle_stats.1,
            approved_vehicles: vehicle_stats.2,
            total_parking_spots: parking_stats.0,
            available_parking_spots: parking_stats.1,
        })
    }
}

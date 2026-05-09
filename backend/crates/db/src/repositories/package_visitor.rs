//! Package and Visitor repository for Epic 58.
//!
//! Handles all database operations for package tracking and visitor management.

use crate::models::{
    package_status, visitor_status, AccessCodeVerification, BuildingPackageSettings,
    BuildingVisitorSettings, CheckInVisitor, CheckOutVisitor, CreatePackage, CreateVisitor,
    Package, PackageQuery, PackageStatistics, PackageSummary, PackageWithDetails, PickupPackage,
    ReceivePackage, UpdateBuildingPackageSettings, UpdateBuildingVisitorSettings, UpdatePackage,
    UpdateVisitor, Visitor, VisitorQuery, VisitorStatistics, VisitorSummary, VisitorWithDetails,
};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Repository for package and visitor management operations.
#[derive(Clone)]
pub struct PackageVisitorRepository {
    pool: PgPool,
}

impl PackageVisitorRepository {
    /// Creates a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Package Operations
    // ========================================================================

    /// Creates a new package registration.
    pub async fn create_package(
        &self,
        tenant_id: Uuid,
        resident_id: Uuid,
        data: CreatePackage,
    ) -> Result<Package, sqlx::Error> {
        // Get building_id from unit_id
        let building_id: Uuid =
            sqlx::query_scalar("SELECT building_id FROM units WHERE id = $1 AND tenant_id = $2")
                .bind(data.unit_id)
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;

        sqlx::query_as(
            r#"
            INSERT INTO packages (
                tenant_id, building_id, unit_id, resident_id, tracking_number,
                carrier, carrier_name, description, expected_date, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .bind(data.unit_id)
        .bind(resident_id)
        .bind(&data.tracking_number)
        .bind(&data.carrier)
        .bind(&data.carrier_name)
        .bind(&data.description)
        .bind(data.expected_date)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Gets a package by ID.
    pub async fn get_package(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Package>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM packages WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Gets a package with all details.
    pub async fn get_package_with_details(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<PackageWithDetails>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT
                p.*,
                u.unit_number,
                b.name as building_name,
                usr.display_name as resident_name,
                recv.display_name as received_by_name
            FROM packages p
            LEFT JOIN units u ON p.unit_id = u.id
            LEFT JOIN buildings b ON p.building_id = b.id
            LEFT JOIN users usr ON p.resident_id = usr.id
            LEFT JOIN users recv ON p.received_by = recv.id
            WHERE p.id = $1 AND p.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Lists packages with optional filtering.
    pub async fn list_packages(
        &self,
        tenant_id: Uuid,
        query: PackageQuery,
    ) -> Result<(Vec<PackageSummary>, i64), sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let packages: Vec<PackageSummary> = sqlx::query_as(
            r#"
            SELECT
                p.id,
                p.unit_id,
                u.unit_number,
                p.resident_id,
                usr.display_name as resident_name,
                p.tracking_number,
                p.carrier,
                p.status,
                p.expected_date,
                p.received_at,
                p.created_at
            FROM packages p
            LEFT JOIN units u ON p.unit_id = u.id
            LEFT JOIN users usr ON p.resident_id = usr.id
            WHERE p.tenant_id = $1
                AND ($2::uuid IS NULL OR p.building_id = $2)
                AND ($3::uuid IS NULL OR p.unit_id = $3)
                AND ($4::uuid IS NULL OR p.resident_id = $4)
                AND ($5::text IS NULL OR p.status = $5)
                AND ($6::text IS NULL OR p.carrier = $6)
                AND ($7::date IS NULL OR p.created_at::date >= $7)
                AND ($8::date IS NULL OR p.created_at::date <= $8)
            ORDER BY p.created_at DESC
            LIMIT $9 OFFSET $10
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.resident_id)
        .bind(&query.status)
        .bind(&query.carrier)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM packages p
            WHERE p.tenant_id = $1
                AND ($2::uuid IS NULL OR p.building_id = $2)
                AND ($3::uuid IS NULL OR p.unit_id = $3)
                AND ($4::uuid IS NULL OR p.resident_id = $4)
                AND ($5::text IS NULL OR p.status = $5)
                AND ($6::text IS NULL OR p.carrier = $6)
                AND ($7::date IS NULL OR p.created_at::date >= $7)
                AND ($8::date IS NULL OR p.created_at::date <= $8)
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.resident_id)
        .bind(&query.status)
        .bind(&query.carrier)
        .bind(query.from_date)
        .bind(query.to_date)
        .fetch_one(&self.pool)
        .await?;

        Ok((packages, total.0))
    }

    /// Updates a package.
    pub async fn update_package(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        data: UpdatePackage,
    ) -> Result<Option<Package>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE packages SET
                tracking_number = COALESCE($3, tracking_number),
                carrier = COALESCE($4, carrier),
                carrier_name = COALESCE($5, carrier_name),
                description = COALESCE($6, description),
                expected_date = COALESCE($7, expected_date),
                storage_location = COALESCE($8, storage_location),
                notes = COALESCE($9, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&data.tracking_number)
        .bind(&data.carrier)
        .bind(&data.carrier_name)
        .bind(&data.description)
        .bind(data.expected_date)
        .bind(&data.storage_location)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Marks a package as received by staff.
    pub async fn receive_package(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        staff_id: Uuid,
        data: ReceivePackage,
    ) -> Result<Option<Package>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE packages SET
                status = $3,
                received_at = NOW(),
                received_by = $4,
                storage_location = COALESCE($5, storage_location),
                photo_url = COALESCE($6, photo_url),
                notes = COALESCE($7, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'expected'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(package_status::RECEIVED)
        .bind(staff_id)
        .bind(&data.storage_location)
        .bind(&data.photo_url)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Marks a package as picked up.
    pub async fn pickup_package(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        resident_id: Uuid,
        data: PickupPackage,
    ) -> Result<Option<Package>, sqlx::Error> {
        let picked_up_by = data.picked_up_by.unwrap_or(resident_id);
        sqlx::query_as(
            r#"
            UPDATE packages SET
                status = $3,
                picked_up_at = NOW(),
                picked_up_by = $4,
                notes = COALESCE($5, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status IN ('received', 'notified')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(package_status::PICKED_UP)
        .bind(picked_up_by)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a package.
    pub async fn delete_package(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM packages WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Gets package statistics for a building.
    pub async fn get_package_statistics(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
    ) -> Result<PackageStatistics, sqlx::Error> {
        let stats: (i64, i64, i64, i64, i64, Option<f64>) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_packages,
                COUNT(*) FILTER (WHERE status = 'expected') as expected_packages,
                COUNT(*) FILTER (WHERE status = 'received' OR status = 'notified') as received_packages,
                COUNT(*) FILTER (WHERE status = 'picked_up') as picked_up_packages,
                COUNT(*) FILTER (WHERE status = 'unclaimed') as unclaimed_packages,
                AVG(EXTRACT(EPOCH FROM (picked_up_at - received_at)) / 3600)
                    FILTER (WHERE picked_up_at IS NOT NULL AND received_at IS NOT NULL) as avg_pickup_time_hours
            FROM packages
            WHERE tenant_id = $1 AND building_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(PackageStatistics {
            total_packages: stats.0,
            expected_packages: stats.1,
            received_packages: stats.2,
            picked_up_packages: stats.3,
            unclaimed_packages: stats.4,
            avg_pickup_time_hours: stats.5,
        })
    }

    // ========================================================================
    // Visitor Operations
    // ========================================================================

    /// Generates a random access code.
    ///
    /// Uses OS CSPRNG directly; visitor codes are short-lived shared secrets.
    fn generate_access_code(length: usize) -> String {
        use rand::Rng;
        const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::rngs::OsRng;
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARS.len());
                CHARS[idx] as char
            })
            .collect()
    }

    /// Generates a unique access code, checking for collisions with active visitors.
    async fn generate_unique_access_code(
        &self,
        building_id: Uuid,
        length: usize,
    ) -> Result<String, sqlx::Error> {
        const MAX_ATTEMPTS: i32 = 10;

        for _ in 0..MAX_ATTEMPTS {
            let code = Self::generate_access_code(length);

            // Check if code already exists for an active visitor in this building
            let exists: Option<bool> = sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM visitors
                    WHERE access_code = $1
                    AND building_id = $2
                    AND status IN ('pending', 'checked_in')
                )
                "#,
            )
            .bind(&code)
            .bind(building_id)
            .fetch_one(&self.pool)
            .await?;

            if !exists.unwrap_or(false) {
                return Ok(code);
            }
        }

        // If we exhausted attempts, return an error
        Err(sqlx::Error::Protocol(
            "Failed to generate unique access code after maximum attempts".into(),
        ))
    }

    /// Creates a new visitor registration.
    pub async fn create_visitor(
        &self,
        tenant_id: Uuid,
        host_id: Uuid,
        data: CreateVisitor,
    ) -> Result<Visitor, sqlx::Error> {
        // Get building_id from unit_id
        let building_id: Uuid =
            sqlx::query_scalar("SELECT building_id FROM units WHERE id = $1 AND tenant_id = $2")
                .bind(data.unit_id)
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;

        // Get visitor settings for code length and validity
        let settings: BuildingVisitorSettings = self
            .get_visitor_settings(tenant_id, building_id)
            .await?
            .unwrap_or_else(|| BuildingVisitorSettings {
                id: Uuid::nil(),
                tenant_id,
                building_id,
                default_code_validity_hours: 24,
                code_length: 6,
                require_purpose: false,
                max_visitors_per_day_per_unit: None,
                max_advance_registration_days: 30,
                notify_host_on_checkin: true,
                send_visitor_instructions: true,
                require_id_verification: false,
                require_photo: false,
                visitor_instructions: None,
                staff_instructions: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });

        // Generate unique access code with retry logic
        let access_code = self
            .generate_unique_access_code(building_id, settings.code_length as usize)
            .await?;
        let expires_at = Utc::now() + Duration::hours(settings.default_code_validity_hours as i64);

        sqlx::query_as(
            r#"
            INSERT INTO visitors (
                tenant_id, building_id, unit_id, host_id, visitor_name,
                visitor_email, visitor_phone, company_name, purpose, purpose_notes,
                access_code, access_code_expires_at, expected_arrival, expected_departure,
                vehicle_license_plate, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .bind(data.unit_id)
        .bind(host_id)
        .bind(&data.visitor_name)
        .bind(&data.visitor_email)
        .bind(&data.visitor_phone)
        .bind(&data.company_name)
        .bind(&data.purpose)
        .bind(&data.purpose_notes)
        .bind(&access_code)
        .bind(expires_at)
        .bind(data.expected_arrival)
        .bind(data.expected_departure)
        .bind(&data.vehicle_license_plate)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Gets a visitor by ID.
    pub async fn get_visitor(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Visitor>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM visitors WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Gets a visitor with all details.
    pub async fn get_visitor_with_details(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<VisitorWithDetails>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT
                v.*,
                u.unit_number,
                b.name as building_name,
                h.display_name as host_name,
                ci.display_name as checked_in_by_name
            FROM visitors v
            LEFT JOIN units u ON v.unit_id = u.id
            LEFT JOIN buildings b ON v.building_id = b.id
            LEFT JOIN users h ON v.host_id = h.id
            LEFT JOIN users ci ON v.checked_in_by = ci.id
            WHERE v.id = $1 AND v.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Lists visitors with optional filtering.
    pub async fn list_visitors(
        &self,
        tenant_id: Uuid,
        query: VisitorQuery,
    ) -> Result<(Vec<VisitorSummary>, i64), sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let today_start = if query.today_only.unwrap_or(false) {
            Some(Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap())
        } else {
            None
        };

        let today_end = if query.today_only.unwrap_or(false) {
            Some(Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap())
        } else {
            None
        };

        let visitors: Vec<VisitorSummary> = sqlx::query_as(
            r#"
            SELECT
                v.id,
                v.unit_id,
                u.unit_number,
                v.host_id,
                h.display_name as host_name,
                v.visitor_name,
                v.purpose,
                v.expected_arrival,
                v.status,
                v.access_code,
                v.created_at
            FROM visitors v
            LEFT JOIN units u ON v.unit_id = u.id
            LEFT JOIN users h ON v.host_id = h.id
            WHERE v.tenant_id = $1
                AND ($2::uuid IS NULL OR v.building_id = $2)
                AND ($3::uuid IS NULL OR v.unit_id = $3)
                AND ($4::uuid IS NULL OR v.host_id = $4)
                AND ($5::text IS NULL OR v.status = $5)
                AND ($6::text IS NULL OR v.purpose = $6)
                AND ($7::timestamptz IS NULL OR v.expected_arrival >= $7)
                AND ($8::timestamptz IS NULL OR v.expected_arrival <= $8)
                AND ($9::timestamp IS NULL OR v.expected_arrival::date = $9::date)
                AND ($10::timestamp IS NULL OR v.expected_arrival::date = $10::date)
            ORDER BY v.expected_arrival ASC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.host_id)
        .bind(&query.status)
        .bind(&query.purpose)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(today_start)
        .bind(today_end)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM visitors v
            WHERE v.tenant_id = $1
                AND ($2::uuid IS NULL OR v.building_id = $2)
                AND ($3::uuid IS NULL OR v.unit_id = $3)
                AND ($4::uuid IS NULL OR v.host_id = $4)
                AND ($5::text IS NULL OR v.status = $5)
                AND ($6::text IS NULL OR v.purpose = $6)
                AND ($7::timestamptz IS NULL OR v.expected_arrival >= $7)
                AND ($8::timestamptz IS NULL OR v.expected_arrival <= $8)
                AND ($9::timestamp IS NULL OR v.expected_arrival::date = $9::date)
                AND ($10::timestamp IS NULL OR v.expected_arrival::date = $10::date)
            "#,
        )
        .bind(tenant_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.host_id)
        .bind(&query.status)
        .bind(&query.purpose)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(today_start)
        .bind(today_end)
        .fetch_one(&self.pool)
        .await?;

        Ok((visitors, total.0))
    }

    /// Updates a visitor registration.
    pub async fn update_visitor(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        data: UpdateVisitor,
    ) -> Result<Option<Visitor>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE visitors SET
                visitor_name = COALESCE($3, visitor_name),
                visitor_email = COALESCE($4, visitor_email),
                visitor_phone = COALESCE($5, visitor_phone),
                company_name = COALESCE($6, company_name),
                purpose = COALESCE($7, purpose),
                purpose_notes = COALESCE($8, purpose_notes),
                expected_arrival = COALESCE($9, expected_arrival),
                expected_departure = COALESCE($10, expected_departure),
                vehicle_license_plate = COALESCE($11, vehicle_license_plate),
                notes = COALESCE($12, notes),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&data.visitor_name)
        .bind(&data.visitor_email)
        .bind(&data.visitor_phone)
        .bind(&data.company_name)
        .bind(&data.purpose)
        .bind(&data.purpose_notes)
        .bind(data.expected_arrival)
        .bind(data.expected_departure)
        .bind(&data.vehicle_license_plate)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Checks in a visitor.
    pub async fn check_in_visitor(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        staff_id: Uuid,
        _data: CheckInVisitor,
    ) -> Result<Option<Visitor>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE visitors SET
                status = $3,
                checked_in_at = NOW(),
                checked_in_by = $4,
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(visitor_status::CHECKED_IN)
        .bind(staff_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Checks out a visitor.
    pub async fn check_out_visitor(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        staff_id: Uuid,
        _data: CheckOutVisitor,
    ) -> Result<Option<Visitor>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE visitors SET
                status = $3,
                checked_out_at = NOW(),
                checked_out_by = $4,
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'checked_in'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(visitor_status::CHECKED_OUT)
        .bind(staff_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Cancels a visitor registration.
    pub async fn cancel_visitor(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Visitor>, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE visitors SET
                status = $3,
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(visitor_status::CANCELLED)
        .fetch_optional(&self.pool)
        .await
    }

    /// Verifies an access code.
    pub async fn verify_access_code(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
        access_code: &str,
    ) -> Result<AccessCodeVerification, sqlx::Error> {
        let visitor: Option<VisitorSummary> = sqlx::query_as(
            r#"
            SELECT
                v.id,
                v.unit_id,
                u.unit_number,
                v.host_id,
                h.display_name as host_name,
                v.visitor_name,
                v.purpose,
                v.expected_arrival,
                v.status,
                v.access_code,
                v.created_at
            FROM visitors v
            LEFT JOIN units u ON v.unit_id = u.id
            LEFT JOIN users h ON v.host_id = h.id
            WHERE v.tenant_id = $1
                AND v.building_id = $2
                AND v.access_code = $3
                AND v.status = 'pending'
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .bind(access_code)
        .fetch_optional(&self.pool)
        .await?;

        match visitor {
            Some(v) => {
                // Check if code is expired
                let now = Utc::now();
                let expires_at: DateTime<chrono::Utc> =
                    sqlx::query_scalar("SELECT access_code_expires_at FROM visitors WHERE id = $1")
                        .bind(v.id)
                        .fetch_one(&self.pool)
                        .await?;

                if now > expires_at {
                    Ok(AccessCodeVerification {
                        valid: false,
                        visitor: None,
                        message: "Access code has expired".to_string(),
                    })
                } else {
                    Ok(AccessCodeVerification {
                        valid: true,
                        visitor: Some(v),
                        message: "Access code is valid".to_string(),
                    })
                }
            }
            None => Ok(AccessCodeVerification {
                valid: false,
                visitor: None,
                message: "Invalid access code".to_string(),
            }),
        }
    }

    /// Gets visitor statistics for a building.
    pub async fn get_visitor_statistics(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
    ) -> Result<VisitorStatistics, sqlx::Error> {
        let stats: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE expected_arrival::date = CURRENT_DATE) as total_visitors_today,
                COUNT(*) FILTER (WHERE status = 'pending' AND expected_arrival::date = CURRENT_DATE) as pending_arrivals,
                COUNT(*) FILTER (WHERE status = 'checked_in') as checked_in_now,
                COUNT(*) FILTER (WHERE expected_arrival >= date_trunc('week', CURRENT_DATE)) as total_this_week,
                COUNT(*) FILTER (WHERE expected_arrival >= date_trunc('month', CURRENT_DATE)) as total_this_month
            FROM visitors
            WHERE tenant_id = $1 AND building_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(VisitorStatistics {
            total_visitors_today: stats.0,
            pending_arrivals: stats.1,
            checked_in_now: stats.2,
            total_this_week: stats.3,
            total_this_month: stats.4,
        })
    }

    /// Deletes a visitor registration.
    pub async fn delete_visitor(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM visitors WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Settings Operations
    // ========================================================================

    /// Gets package settings for a building.
    pub async fn get_package_settings(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
    ) -> Result<Option<BuildingPackageSettings>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM building_package_settings WHERE tenant_id = $1 AND building_id = $2",
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Upserts package settings for a building.
    pub async fn upsert_package_settings(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
        data: UpdateBuildingPackageSettings,
    ) -> Result<BuildingPackageSettings, sqlx::Error> {
        // Validate settings
        if let Some(days) = data.max_storage_days {
            if !(1..=365).contains(&days) {
                return Err(sqlx::Error::Protocol(
                    "max_storage_days must be between 1 and 365".into(),
                ));
            }
        }
        if let Some(reminder) = data.send_reminder_after_days {
            let max_days = data.max_storage_days.unwrap_or(7);
            if reminder < 0 || reminder > max_days {
                return Err(sqlx::Error::Protocol(
                    "send_reminder_after_days must be non-negative and not exceed max_storage_days"
                        .into(),
                ));
            }
        }

        sqlx::query_as(
            r#"
            INSERT INTO building_package_settings (
                tenant_id, building_id, max_storage_days, send_reminder_after_days,
                require_photo_on_receipt, allow_resident_self_pickup, notify_on_arrival,
                send_daily_summary, storage_instructions
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (tenant_id, building_id) DO UPDATE SET
                max_storage_days = COALESCE($3, building_package_settings.max_storage_days),
                send_reminder_after_days = COALESCE($4, building_package_settings.send_reminder_after_days),
                require_photo_on_receipt = COALESCE($5, building_package_settings.require_photo_on_receipt),
                allow_resident_self_pickup = COALESCE($6, building_package_settings.allow_resident_self_pickup),
                notify_on_arrival = COALESCE($7, building_package_settings.notify_on_arrival),
                send_daily_summary = COALESCE($8, building_package_settings.send_daily_summary),
                storage_instructions = COALESCE($9, building_package_settings.storage_instructions),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .bind(data.max_storage_days.unwrap_or(7))
        .bind(data.send_reminder_after_days.unwrap_or(3))
        .bind(data.require_photo_on_receipt.unwrap_or(false))
        .bind(data.allow_resident_self_pickup.unwrap_or(true))
        .bind(data.notify_on_arrival.unwrap_or(true))
        .bind(data.send_daily_summary.unwrap_or(false))
        .bind(&data.storage_instructions)
        .fetch_one(&self.pool)
        .await
    }

    /// Gets visitor settings for a building.
    pub async fn get_visitor_settings(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
    ) -> Result<Option<BuildingVisitorSettings>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM building_visitor_settings WHERE tenant_id = $1 AND building_id = $2",
        )
        .bind(tenant_id)
        .bind(building_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Upserts visitor settings for a building.
    pub async fn upsert_visitor_settings(
        &self,
        tenant_id: Uuid,
        building_id: Uuid,
        data: UpdateBuildingVisitorSettings,
    ) -> Result<BuildingVisitorSettings, sqlx::Error> {
        // Validate settings
        if let Some(code_length) = data.code_length {
            if !(4..=12).contains(&code_length) {
                return Err(sqlx::Error::Protocol(
                    "code_length must be between 4 and 12".into(),
                ));
            }
        }
        if let Some(hours) = data.default_code_validity_hours {
            if !(1..=8760).contains(&hours) {
                // max 1 year
                return Err(sqlx::Error::Protocol(
                    "default_code_validity_hours must be between 1 and 8760 (1 year)".into(),
                ));
            }
        }
        if let Some(days) = data.max_advance_registration_days {
            if !(1..=365).contains(&days) {
                return Err(sqlx::Error::Protocol(
                    "max_advance_registration_days must be between 1 and 365".into(),
                ));
            }
        }
        if let Some(max_visitors) = data.max_visitors_per_day_per_unit {
            if max_visitors < 1 {
                return Err(sqlx::Error::Protocol(
                    "max_visitors_per_day_per_unit must be at least 1".into(),
                ));
            }
        }

        sqlx::query_as(
            r#"
            INSERT INTO building_visitor_settings (
                tenant_id, building_id, default_code_validity_hours, code_length,
                require_purpose, max_visitors_per_day_per_unit, max_advance_registration_days,
                notify_host_on_checkin, send_visitor_instructions, require_id_verification,
                require_photo, visitor_instructions, staff_instructions
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (tenant_id, building_id) DO UPDATE SET
                default_code_validity_hours = COALESCE($3, building_visitor_settings.default_code_validity_hours),
                code_length = COALESCE($4, building_visitor_settings.code_length),
                require_purpose = COALESCE($5, building_visitor_settings.require_purpose),
                max_visitors_per_day_per_unit = COALESCE($6, building_visitor_settings.max_visitors_per_day_per_unit),
                max_advance_registration_days = COALESCE($7, building_visitor_settings.max_advance_registration_days),
                notify_host_on_checkin = COALESCE($8, building_visitor_settings.notify_host_on_checkin),
                send_visitor_instructions = COALESCE($9, building_visitor_settings.send_visitor_instructions),
                require_id_verification = COALESCE($10, building_visitor_settings.require_id_verification),
                require_photo = COALESCE($11, building_visitor_settings.require_photo),
                visitor_instructions = COALESCE($12, building_visitor_settings.visitor_instructions),
                staff_instructions = COALESCE($13, building_visitor_settings.staff_instructions),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(building_id)
        .bind(data.default_code_validity_hours.unwrap_or(24))
        .bind(data.code_length.unwrap_or(6))
        .bind(data.require_purpose.unwrap_or(false))
        .bind(data.max_visitors_per_day_per_unit)
        .bind(data.max_advance_registration_days.unwrap_or(30))
        .bind(data.notify_host_on_checkin.unwrap_or(true))
        .bind(data.send_visitor_instructions.unwrap_or(true))
        .bind(data.require_id_verification.unwrap_or(false))
        .bind(data.require_photo.unwrap_or(false))
        .bind(&data.visitor_instructions)
        .bind(&data.staff_instructions)
        .fetch_one(&self.pool)
        .await
    }
}

//! Entity factory functions for seeding.
//!
//! Provides factory methods for creating users, organizations, buildings,
//! units, assignments, faults, votes, and announcements in the database.

use crate::DbPool;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use rand::rngs::OsRng;
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

use super::data::{SeedBuilding, SeedUnit};

/// Factory for creating seed entities in the database.
pub struct SeedFactories<'a> {
    pool: &'a DbPool,
}

impl<'a> SeedFactories<'a> {
    /// Create a new factory with the given pool.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Hash a password using Argon2id.
    ///
    /// This matches the password hashing used in AuthService.
    pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    /// Create a user with a hashed password.
    ///
    /// Returns the user's UUID.
    pub async fn create_user(
        &self,
        email: &str,
        name: &str,
        password_hash: &str,
        phone: Option<&str>,
        is_super_admin: bool,
        locale: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (
                email, password_hash, name, phone, status,
                email_verified_at, is_super_admin, locale
            )
            VALUES ($1, $2, $3, $4, 'active', NOW(), $5, $6)
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .bind(phone)
        .bind(is_super_admin)
        .bind(locale)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create an organization.
    ///
    /// This triggers automatic creation of default roles.
    /// Returns the organization's UUID.
    pub async fn create_organization(
        &self,
        name: &str,
        slug: &str,
        contact_email: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO organizations (name, slug, contact_email, status, settings)
            VALUES ($1, $2, $3, 'active', '{}'::jsonb)
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(contact_email)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Add a user to an organization with a specific role.
    ///
    /// The role_type should match one of the system roles created
    /// when the organization was created.
    ///
    /// Returns the membership UUID.
    pub async fn add_organization_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role_type: &str,
    ) -> Result<Uuid, sqlx::Error> {
        // Get the role_id for this role type in the organization
        let role_row = sqlx::query(
            r#"
            SELECT id FROM roles
            WHERE organization_id = $1 AND name = $2
            "#,
        )
        .bind(org_id)
        .bind(role_type)
        .fetch_optional(self.pool)
        .await?;

        let role_id: Option<Uuid> = role_row.map(|r| r.get("id"));

        let row = sqlx::query(
            r#"
            INSERT INTO organization_members (
                organization_id, user_id, role_id, role_type,
                status, joined_at
            )
            VALUES ($1, $2, $3, $4, 'active', NOW())
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role_id)
        .bind(role_type)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a building in an organization.
    ///
    /// Returns the building's UUID.
    pub async fn create_building(
        &self,
        org_id: Uuid,
        building: &SeedBuilding,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO buildings (
                organization_id, name, street, city, postal_code, country,
                total_floors, year_built, total_entrances, status,
                amenities, contacts, settings
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 'active', '[]'::jsonb, '[]'::jsonb, '{}'::jsonb)
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(building.name)
        .bind(building.street)
        .bind(building.city)
        .bind(building.postal_code)
        .bind(building.country)
        .bind(building.total_floors)
        .bind(building.year_built)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a unit in a building.
    ///
    /// Returns the unit's UUID.
    pub async fn create_unit(
        &self,
        building_id: Uuid,
        unit: &SeedUnit,
    ) -> Result<Uuid, sqlx::Error> {
        let size_sqm = unit.size_sqm.map(Decimal::from);

        let row = sqlx::query(
            r#"
            INSERT INTO units (
                building_id, designation, floor, unit_type,
                size_sqm, rooms, ownership_share, occupancy_status, status, settings
            )
            VALUES ($1, $2, $3, $4, $5, $6, 100.00, 'unknown', 'active', '{}'::jsonb)
            RETURNING id
            "#,
        )
        .bind(building_id)
        .bind(unit.designation)
        .bind(unit.floor)
        .bind(unit.unit_type)
        .bind(size_sqm)
        .bind(unit.rooms)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Assign a user as a resident of a unit.
    ///
    /// Returns the unit_resident UUID.
    pub async fn assign_unit_resident(
        &self,
        unit_id: Uuid,
        user_id: Uuid,
        resident_type: &str,
        is_primary: bool,
    ) -> Result<Uuid, sqlx::Error> {
        let today = Utc::now().date_naive();
        // from_ymd_opt should never return None for the current year; if it does, fail loudly
        let start_date = NaiveDate::from_ymd_opt(today.year(), 1, 1)
            .expect("current year must be a valid NaiveDate");

        let row = sqlx::query(
            r#"
            INSERT INTO unit_residents (
                unit_id, user_id, resident_type, is_primary,
                start_date, receives_notifications, receives_mail
            )
            VALUES ($1, $2, $3::resident_type, $4, $5, true, true)
            RETURNING id
            "#,
        )
        .bind(unit_id)
        .bind(user_id)
        .bind(resident_type)
        .bind(is_primary)
        .bind(start_date)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Update the occupancy status of a unit.
    pub async fn update_unit_occupancy(
        &self,
        unit_id: Uuid,
        occupancy_status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE units SET occupancy_status = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(occupancy_status)
        .bind(unit_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Create a listing for a building unit.
    ///
    /// Fetches the building address automatically.
    /// `published_at` is derived from status: "active" → 14 days ago, "archived" → 60 days ago.
    ///
    /// Returns the listing's UUID.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_listing(
        &self,
        org_id: Uuid,
        building_id: Uuid,
        unit_id: Option<Uuid>,
        created_by: Uuid,
        title: &str,
        description: &str,
        transaction_type: &str,
        property_type: &str,
        status: &str,
        price: i64,
        is_negotiable: bool,
        rooms: Option<i32>,
        bathrooms: Option<i32>,
        features: &[&str],
    ) -> Result<Uuid, sqlx::Error> {
        let building = sqlx::query(
            "SELECT street, city, postal_code, country, total_floors FROM buildings WHERE id = $1",
        )
        .bind(building_id)
        .fetch_one(self.pool)
        .await?;

        let street: String = building.get("street");
        let city: String = building.get("city");
        let postal_code: String = building.get("postal_code");
        let country: String = building.get("country");
        let total_floors: i32 = building.get("total_floors");

        let (floor, size_sqm) = if let Some(uid) = unit_id {
            let unit = sqlx::query("SELECT floor, size_sqm FROM units WHERE id = $1")
                .bind(uid)
                .fetch_one(self.pool)
                .await?;
            let f: i32 = unit.get("floor");
            let s: Option<Decimal> = unit.get("size_sqm");
            (Some(f), s)
        } else {
            (None, None)
        };

        let price_decimal = Decimal::from(price);

        let features_json = serde_json::Value::Array(
            features
                .iter()
                .map(|f| serde_json::Value::String(f.to_string()))
                .collect(),
        );

        let now = Utc::now();
        let published_at = match status {
            "active" => Some(now - Duration::days(14)),
            "archived" => Some(now - Duration::days(60)),
            _ => None,
        };

        let row = sqlx::query(
            r#"
            INSERT INTO listings (
                organization_id, unit_id, created_by,
                status, transaction_type,
                title, description, property_type,
                size_sqm, rooms, bathrooms, floor, total_floors,
                street, city, postal_code, country,
                price, currency, is_negotiable,
                features, published_at
            )
            VALUES (
                $1, $2, $3,
                $4, $5,
                $6, $7, $8,
                $9, $10, $11, $12, $13,
                $14, $15, $16, $17,
                $18, 'EUR', $19,
                $20, $21
            )
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(unit_id)
        .bind(created_by)
        .bind(status)
        .bind(transaction_type)
        .bind(title)
        .bind(description)
        .bind(property_type)
        .bind(size_sqm)
        .bind(rooms)
        .bind(bathrooms)
        .bind(floor)
        .bind(total_floors)
        .bind(&street)
        .bind(&city)
        .bind(&postal_code)
        .bind(&country)
        .bind(price_decimal)
        .bind(is_negotiable)
        .bind(features_json)
        .bind(published_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a portal user (Reality Portal, separate from PPT users).
    ///
    /// Returns the portal user's UUID.
    pub async fn create_portal_user(
        &self,
        email: &str,
        name: &str,
        password_hash: &str,
        pm_user_id: Option<Uuid>,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO portal_users (
                email, name, password_hash, pm_user_id,
                provider, email_verified, locale
            )
            VALUES ($1, $2, $3, $4, 'local', TRUE, 'sk')
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(name)
        .bind(password_hash)
        .bind(pm_user_id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a listing inquiry from a portal user to a realtor.
    ///
    /// `read_at` and `responded_at` are derived from status.
    ///
    /// Returns the inquiry's UUID.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_inquiry(
        &self,
        listing_id: Uuid,
        realtor_id: Uuid,
        user_id: Uuid,
        name: &str,
        email: &str,
        phone: Option<&str>,
        message: &str,
        inquiry_type: &str,
        preferred_contact: &str,
        status: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let now = Utc::now();
        let (read_at, responded_at) = match status {
            "responded" => (
                Some(now - Duration::hours(4)),
                Some(now - Duration::hours(2)),
            ),
            "read" => (Some(now - Duration::hours(1)), None),
            _ => (None, None),
        };

        let row = sqlx::query(
            r#"
            INSERT INTO listing_inquiries (
                listing_id, realtor_id, user_id,
                name, email, phone,
                message, inquiry_type, preferred_contact,
                status, read_at, responded_at, source
            )
            VALUES (
                $1, $2, $3,
                $4, $5, $6,
                $7, $8, $9,
                $10, $11, $12, 'website'
            )
            RETURNING id
            "#,
        )
        .bind(listing_id)
        .bind(realtor_id)
        .bind(user_id)
        .bind(name)
        .bind(email)
        .bind(phone)
        .bind(message)
        .bind(inquiry_type)
        .bind(preferred_contact)
        .bind(status)
        .bind(read_at)
        .bind(responded_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a reply message in an inquiry thread.
    ///
    /// Returns the message's UUID.
    pub async fn create_inquiry_message(
        &self,
        inquiry_id: Uuid,
        sender_type: &str,
        sender_id: Uuid,
        message: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO inquiry_messages (inquiry_id, sender_type, sender_id, message)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(inquiry_id)
        .bind(sender_type)
        .bind(sender_id)
        .bind(message)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Save a listing as a favorite for a portal user.
    pub async fn create_portal_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO portal_favorites (user_id, listing_id)
            VALUES ($1, $2)
            ON CONFLICT (user_id, listing_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(listing_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Create a fault report.
    ///
    /// Status-specific timestamps (triaged_at, assigned_at, resolved_at, confirmed_at)
    /// are computed automatically based on the given status, spaced out in the past.
    ///
    /// Returns the fault's UUID.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_fault(
        &self,
        org_id: Uuid,
        building_id: Uuid,
        unit_id: Option<Uuid>,
        reporter_id: Uuid,
        title: &str,
        description: &str,
        location_description: Option<&str>,
        category: &str,
        priority: &str,
        status: &str,
        assigned_to: Option<Uuid>,
        resolved_by: Option<Uuid>,
        confirmed_by: Option<Uuid>,
        rating: Option<i32>,
    ) -> Result<Uuid, sqlx::Error> {
        let now = Utc::now();

        // Derive timestamps from status so the seed data is chronologically consistent.
        let (triaged_at, assigned_at, resolved_at, confirmed_at) = match status {
            "closed" => (
                Some(now - Duration::days(26)),
                Some(now - Duration::days(25)),
                Some(now - Duration::days(20)),
                Some(now - Duration::days(18)),
            ),
            "in_progress" => (
                Some(now - Duration::days(5)),
                Some(now - Duration::days(4)),
                None,
                None,
            ),
            "triaged" => (
                Some(now - Duration::days(2)),
                assigned_to.map(|_| now - Duration::days(1)),
                None,
                None,
            ),
            "resolved" => (
                Some(now - Duration::days(14)),
                Some(now - Duration::days(13)),
                Some(now - Duration::days(7)),
                None,
            ),
            "scheduled" => (
                Some(now - Duration::days(3)),
                Some(now - Duration::days(2)),
                None,
                None,
            ),
            _ => (None, None, None, None), // "new"
        };

        // Use assigned_to as triaged_by (same person triaged and was assigned).
        let triaged_by = assigned_to;

        let row = sqlx::query(
            r#"
            INSERT INTO faults (
                organization_id, building_id, unit_id, reporter_id,
                title, description, location_description,
                category, priority, status,
                assigned_to, assigned_at,
                triaged_by, triaged_at,
                resolved_at, resolved_by,
                confirmed_at, confirmed_by, rating
            )
            VALUES (
                $1, $2, $3, $4,
                $5, $6, $7,
                $8::fault_category, $9::fault_priority, $10::fault_status,
                $11, $12,
                $13, $14,
                $15, $16,
                $17, $18, $19
            )
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(unit_id)
        .bind(reporter_id)
        .bind(title)
        .bind(description)
        .bind(location_description)
        .bind(category)
        .bind(priority)
        .bind(status)
        .bind(assigned_to)
        .bind(assigned_at)
        .bind(triaged_by)
        .bind(triaged_at)
        .bind(resolved_at)
        .bind(resolved_by)
        .bind(confirmed_at)
        .bind(confirmed_by)
        .bind(rating)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a vote for a building.
    ///
    /// Timestamps (start_at, end_at, published_at) are derived from the status:
    /// - "closed": voting window 37–30 days ago, published 37 days ago
    /// - "active": started 1 day ago, ends in 6 days, published 1 day ago
    /// - "draft": end_at 7 days in the future, not yet published
    ///
    /// Returns the vote's UUID.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_vote(
        &self,
        org_id: Uuid,
        building_id: Uuid,
        title: &str,
        description: Option<&str>,
        status: &str,
        created_by: Uuid,
        quorum_type: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let now = Utc::now();

        let (start_at, end_at, published_at, published_by) = match status {
            "closed" => (
                Some(now - Duration::days(37)),
                now - Duration::days(30),
                Some(now - Duration::days(37)),
                Some(created_by),
            ),
            "active" => (
                Some(now - Duration::days(1)),
                now + Duration::days(6),
                Some(now - Duration::days(1)),
                Some(created_by),
            ),
            _ => (None, now + Duration::days(7), None, None), // "draft"
        };

        let row = sqlx::query(
            r#"
            INSERT INTO votes (
                organization_id, building_id,
                title, description,
                start_at, end_at,
                status, quorum_type,
                allow_delegation, anonymous_voting,
                created_by, published_by, published_at
            )
            VALUES (
                $1, $2,
                $3, $4,
                $5, $6,
                $7::vote_status, $8::quorum_type,
                TRUE, FALSE,
                $9, $10, $11
            )
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(title)
        .bind(description)
        .bind(start_at)
        .bind(end_at)
        .bind(status)
        .bind(quorum_type)
        .bind(created_by)
        .bind(published_by)
        .bind(published_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a question for a vote.
    ///
    /// For "yes_no" questions, options are empty (yes/no is implied).
    /// Options for "single_choice" questions are passed in from the seed data.
    /// For "yes_no" questions pass an empty slice.
    ///
    /// Returns the question's UUID.
    pub async fn create_vote_question(
        &self,
        vote_id: Uuid,
        question_text: &str,
        question_type: &str,
        display_order: i32,
        option_texts: &[&str],
    ) -> Result<Uuid, sqlx::Error> {
        let options = serde_json::Value::Array(
            option_texts
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    serde_json::json!({
                        "id": Uuid::new_v4().to_string(),
                        "text": text,
                        "order": i + 1
                    })
                })
                .collect(),
        );

        let row = sqlx::query(
            r#"
            INSERT INTO vote_questions (
                vote_id, question_text, question_type, options, display_order, is_required
            )
            VALUES ($1, $2, $3::vote_question_type, $4, $5, TRUE)
            RETURNING id
            "#,
        )
        .bind(vote_id)
        .bind(question_text)
        .bind(question_type)
        .bind(options)
        .bind(display_order)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create an announcement.
    ///
    /// `target_ids_json` should be a JSON array of UUIDs (as strings) for building/unit targets,
    /// or an empty array for "all" / "roles" targets.
    ///
    /// `published_at` is set automatically based on status:
    /// - "published": 10 days ago
    /// - "archived": 60 days ago
    /// - "draft"/"scheduled": None
    ///
    /// Returns the announcement's UUID.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_announcement(
        &self,
        org_id: Uuid,
        author_id: Uuid,
        title: &str,
        content: &str,
        target_type: &str,
        target_ids: serde_json::Value,
        status: &str,
        pinned: bool,
        acknowledgment_required: bool,
    ) -> Result<Uuid, sqlx::Error> {
        let now = Utc::now();

        let published_at = match status {
            "published" => Some(now - Duration::days(10)),
            "archived" => Some(now - Duration::days(60)),
            _ => None,
        };

        let (pinned_at, pinned_by) = if pinned {
            (Some(published_at.unwrap_or(now)), Some(author_id))
        } else {
            (None, None)
        };

        let row = sqlx::query(
            r#"
            INSERT INTO announcements (
                organization_id, author_id,
                title, content,
                target_type, target_ids,
                status, published_at,
                pinned, pinned_at, pinned_by,
                comments_enabled, acknowledgment_required
            )
            VALUES (
                $1, $2,
                $3, $4,
                $5::announcement_target_type, $6,
                $7::announcement_status, $8,
                $9, $10, $11,
                TRUE, $12
            )
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(author_id)
        .bind(title)
        .bind(content)
        .bind(target_type)
        .bind(target_ids)
        .bind(status)
        .bind(published_at)
        .bind(pinned)
        .bind(pinned_at)
        .bind(pinned_by)
        .bind(acknowledgment_required)
        .fetch_one(self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Delete seed data by email domain pattern.
    ///
    /// `email_domain` is the PPT domain (e.g. "demo-property.test").
    /// `portal_email_domain` is the Reality Portal domain (e.g. "demo-reality.test").
    ///
    /// All operations run within a single transaction for atomicity.
    ///
    /// # Security
    /// Both domains are validated to prevent SQL pattern injection.
    /// Only alphanumeric characters, dots, and hyphens are allowed.
    pub async fn cleanup_seed_data(
        &self,
        email_domain: &str,
        portal_email_domain: &str,
    ) -> Result<CleanupStats, sqlx::Error> {
        let is_valid_domain = |d: &str| {
            d.chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
        };

        if !is_valid_domain(email_domain) {
            return Err(sqlx::Error::Protocol(
                "Invalid email domain: only alphanumeric characters, dots, and hyphens allowed"
                    .to_string(),
            ));
        }
        if !is_valid_domain(portal_email_domain) {
            return Err(sqlx::Error::Protocol(
                "Invalid portal email domain: only alphanumeric characters, dots, and hyphens allowed"
                    .to_string(),
            ));
        }

        let pattern = format!("%@{}", email_domain);
        let portal_pattern = format!("%@{}", portal_email_domain);

        // Use a transaction for atomicity - if any operation fails,
        // all changes are rolled back to maintain consistency
        let mut tx = self.pool.begin().await?;

        // Delete in dependency order to avoid foreign key violations

        // 1. Delete unit_residents for seed users
        let residents_result = sqlx::query(
            r#"
            DELETE FROM unit_residents
            WHERE user_id IN (SELECT id FROM users WHERE email LIKE $1)
            "#,
        )
        .bind(&pattern)
        .execute(&mut *tx)
        .await?;

        // 2. Delete organization_members for seed users
        let members_result = sqlx::query(
            r#"
            DELETE FROM organization_members
            WHERE user_id IN (SELECT id FROM users WHERE email LIKE $1)
            "#,
        )
        .bind(&pattern)
        .execute(&mut *tx)
        .await?;

        // 3. Find organizations created by seed (by contact email pattern)
        let org_ids: Vec<Uuid> = sqlx::query(
            r#"
            SELECT id FROM organizations WHERE contact_email LIKE $1
            "#,
        )
        .bind(&pattern)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| r.get("id"))
        .collect();

        // 4–6. Delete units, buildings, faults, votes, announcements, and roles for each org.
        // Faults cascade-delete when buildings are deleted (ON DELETE CASCADE).
        // Votes cascade-delete when buildings are deleted.
        // Announcements cascade-delete when organizations are deleted.
        let mut units_deleted = 0u64;
        let mut buildings_deleted = 0u64;
        for org_id in &org_ids {
            // Delete units in this org's buildings
            let units_result = sqlx::query(
                r#"
                DELETE FROM units
                WHERE building_id IN (SELECT id FROM buildings WHERE organization_id = $1)
                "#,
            )
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
            units_deleted += units_result.rows_affected();

            // Delete buildings in this org (faults and votes cascade)
            let buildings_result = sqlx::query(
                r#"
                DELETE FROM buildings WHERE organization_id = $1
                "#,
            )
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
            buildings_deleted += buildings_result.rows_affected();

            // Delete roles in this org
            sqlx::query(
                r#"
                DELETE FROM roles WHERE organization_id = $1
                "#,
            )
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        }

        // 7. Delete seed organizations (announcements cascade)
        let orgs_result = sqlx::query(
            r#"
            DELETE FROM organizations WHERE contact_email LIKE $1
            "#,
        )
        .bind(&pattern)
        .execute(&mut *tx)
        .await?;

        // 8. Delete seed users
        let users_result = sqlx::query(
            r#"
            DELETE FROM users WHERE email LIKE $1
            "#,
        )
        .bind(&pattern)
        .execute(&mut *tx)
        .await?;

        // 9. Delete portal_users (listing_inquiries were already cascade-deleted via listings)
        let portal_users_result = sqlx::query(
            r#"
            DELETE FROM portal_users WHERE email LIKE $1
            "#,
        )
        .bind(&portal_pattern)
        .execute(&mut *tx)
        .await?;

        // Commit the transaction - all deletions succeed or none do
        tx.commit().await?;

        Ok(CleanupStats {
            users_deleted: users_result.rows_affected(),
            organizations_deleted: orgs_result.rows_affected(),
            buildings_deleted,
            units_deleted,
            members_deleted: members_result.rows_affected(),
            residents_deleted: residents_result.rows_affected(),
            portal_users_deleted: portal_users_result.rows_affected(),
        })
    }

    /// Check if seed data already exists.
    pub async fn seed_exists(&self, email_domain: &str) -> Result<bool, sqlx::Error> {
        let pattern = format!("%@{}", email_domain);
        let result: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM users WHERE email LIKE $1
            "#,
        )
        .bind(&pattern)
        .fetch_one(self.pool)
        .await?;

        Ok(result.unwrap_or(0) > 0)
    }
}

/// Statistics from cleanup operation.
#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
    pub users_deleted: u64,
    pub organizations_deleted: u64,
    pub buildings_deleted: u64,
    pub units_deleted: u64,
    pub members_deleted: u64,
    pub residents_deleted: u64,
    pub portal_users_deleted: u64,
}

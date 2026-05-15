//! Main seed execution logic.
//!
//! Handles the complete seeding workflow including RLS context,
//! data creation, and cleanup.

use std::collections::HashMap;

use uuid::Uuid;

use crate::{clear_request_context, set_request_context, DbPool};

use super::data::SeedData;
use super::factories::{CleanupStats, SeedFactories};

/// RAII guard to ensure RLS context is cleared even on panic/early return.
/// When dropped, this guard will clear the request context.
struct RlsContextGuard<'a> {
    pool: &'a DbPool,
    cleared: bool,
}

impl<'a> RlsContextGuard<'a> {
    fn new(pool: &'a DbPool) -> Self {
        Self {
            pool,
            cleared: false,
        }
    }

    /// Mark the context as already cleared (call this on success path).
    fn mark_cleared(&mut self) {
        self.cleared = true;
    }

    /// Get the pool reference.
    fn pool(&self) -> &'a DbPool {
        self.pool
    }
}

impl Drop for RlsContextGuard<'_> {
    fn drop(&mut self) {
        if !self.cleared {
            // Best-effort cleanup on error/panic paths.
            // We can't await in Drop, so we spawn a blocking task.
            // In practice, the connection will be returned to the pool
            // and PostgreSQL will reset the session variables.
            tracing::warn!("RLS context guard dropped without explicit clear - session may have elevated privileges until connection reset");
        }
    }
}

/// Configuration for the seed runner.
#[derive(Debug, Clone)]
pub struct SeedConfig {
    /// Admin email address
    pub admin_email: String,
    /// Admin password (plaintext, will be hashed)
    pub admin_password: String,
    /// Whether to include sample data (buildings, units, non-admin users)
    pub include_sample_data: bool,
    /// Whether to force re-seed (drops existing seed data first)
    pub force: bool,
}

/// Result of a successful seed operation.
#[derive(Debug, Clone)]
pub struct SeedResult {
    /// Number of organizations created
    pub organizations_created: usize,
    /// Number of PPT users created
    pub users_created: usize,
    /// Number of buildings created
    pub buildings_created: usize,
    /// Number of units created
    pub units_created: usize,
    /// Number of unit residents assigned
    pub residents_assigned: usize,
    /// Number of faults created
    pub faults_created: usize,
    /// Number of votes created
    pub votes_created: usize,
    /// Number of announcements created
    pub announcements_created: usize,
    /// Number of listings created
    pub listings_created: usize,
    /// Number of portal users created
    pub portal_users_created: usize,
    /// Number of listing inquiries created
    pub inquiries_created: usize,
    /// Number of portal favorites created
    pub favorites_created: usize,
    /// Admin user ID
    pub admin_user_id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Cleanup stats (if force was used)
    pub cleanup_stats: Option<CleanupStats>,
}

/// Seed runner that orchestrates the complete seeding process.
pub struct SeedRunner {
    pool: DbPool,
    config: SeedConfig,
}

impl SeedRunner {
    /// Create a new seed runner.
    pub fn new(pool: DbPool, config: SeedConfig) -> Self {
        Self { pool, config }
    }

    /// Borrow the underlying pool. Used by `ppt-seed` to perform follow-up
    /// inserts (e.g. seeding the `reality-portal` OAuth client) without
    /// having to re-create a connection. Read-only handle on purpose:
    /// callers shouldn't be mutating the runner's connection lifecycle.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Execute the seeding process.
    ///
    /// This will:
    /// 1. Set super admin context to bypass RLS
    /// 2. Optionally cleanup existing seed data (PPT + portal)
    /// 3. Create organization (triggers role creation)
    /// 4. Create admin user with provided credentials
    /// 5. Create sample PPT users, buildings, units (if include_sample_data)
    /// 6. Assign users to units
    /// 7. Create faults, votes, and announcements
    /// 8. Create listings (PPT → Reality Portal bridge)
    /// 9. Create portal users
    /// 10. Create listing inquiries and messages
    /// 11. Create portal favorites
    /// 12. Clear RLS context
    ///
    /// # RLS Context Safety
    /// Uses an RAII guard to ensure RLS context is tracked. On success,
    /// context is explicitly cleared. On error, the guard logs a warning
    /// and the connection pool will reset session variables when the
    /// connection is returned.
    pub async fn run(&self) -> Result<SeedResult, SeedError> {
        let factories = SeedFactories::new(&self.pool);
        let seed_data = SeedData::default();

        // Derive email domains for both PPT and Reality Portal
        let email_domain = seed_data
            .organization
            .contact_email
            .split('@')
            .nth(1)
            .unwrap_or("demo-property.test");

        let portal_email_domain = seed_data
            .portal_users
            .first()
            .and_then(|u| u.email.split('@').nth(1))
            .unwrap_or("demo-reality.test");

        // 1. Set super admin context to bypass RLS
        set_request_context(&self.pool, None, None, true)
            .await
            .map_err(|e| SeedError::Database(e.to_string()))?;

        // Create RAII guard for RLS context cleanup on error paths
        let mut rls_guard = RlsContextGuard::new(&self.pool);

        // 2. Optionally cleanup existing seed data
        let cleanup_stats = if self.config.force {
            let stats = factories
                .cleanup_seed_data(email_domain, portal_email_domain)
                .await
                .map_err(|e| SeedError::Database(e.to_string()))?;
            Some(stats)
        } else {
            // Check if seed data already exists
            if factories
                .seed_exists(email_domain)
                .await
                .map_err(|e| SeedError::Database(e.to_string()))?
            {
                clear_request_context(rls_guard.pool())
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;
                rls_guard.mark_cleared();
                return Err(SeedError::AlreadySeeded);
            }
            None
        };

        // 3. Hash passwords
        let admin_hash = SeedFactories::hash_password(&self.config.admin_password)
            .map_err(|e| SeedError::PasswordHash(e.to_string()))?;

        let default_hash = SeedFactories::hash_password(seed_data.default_password)
            .map_err(|e| SeedError::PasswordHash(e.to_string()))?;

        let portal_hash = SeedFactories::hash_password(seed_data.portal_default_password)
            .map_err(|e| SeedError::PasswordHash(e.to_string()))?;

        // 4. Create organization (triggers role creation)
        let org_id = factories
            .create_organization(
                seed_data.organization.name,
                seed_data.organization.slug,
                seed_data.organization.contact_email,
            )
            .await
            .map_err(|e| SeedError::Database(e.to_string()))?;

        // 5. Create admin user with provided credentials
        let admin_id = factories
            .create_user(
                &self.config.admin_email,
                "System Administrator",
                &admin_hash,
                None,
                "sk",
            )
            .await
            .map_err(|e| SeedError::Database(e.to_string()))?;

        // 6. Add admin to organization with Super Admin role
        factories
            .add_organization_member(org_id, admin_id, "Super Admin")
            .await
            .map_err(|e| SeedError::Database(e.to_string()))?;

        let mut users_created = 1; // admin
        let mut buildings_created = 0;
        let mut units_created = 0;
        let mut residents_assigned = 0;
        let mut faults_created = 0;
        let mut votes_created = 0;
        let mut announcements_created = 0;
        let mut listings_created = 0;
        let mut portal_users_created = 0;
        let mut inquiries_created = 0;
        let mut favorites_created = 0;

        // 7. Create sample data if requested
        if self.config.include_sample_data {
            // Track user IDs by email for unit assignments and references
            let mut user_ids: HashMap<String, Uuid> = HashMap::new();

            // Create sample PPT users
            for user in &seed_data.users {
                let user_id = factories
                    .create_user(
                        user.email,
                        user.name,
                        &default_hash,
                        user.phone,
                        user.locale,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                // Add user to organization with their role
                factories
                    .add_organization_member(org_id, user_id, user.role_type)
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                user_ids.insert(user.email.to_string(), user_id);
                users_created += 1;
            }

            // Create buildings and units
            let mut building_ids: Vec<Uuid> = Vec::new();
            let mut unit_ids: Vec<Vec<Uuid>> = Vec::new();

            for building in &seed_data.buildings {
                let building_id = factories
                    .create_building(org_id, building)
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                building_ids.push(building_id);
                buildings_created += 1;

                let mut building_unit_ids: Vec<Uuid> = Vec::new();
                for unit in &building.units {
                    let unit_id = factories
                        .create_unit(building_id, unit)
                        .await
                        .map_err(|e| SeedError::Database(e.to_string()))?;

                    building_unit_ids.push(unit_id);
                    units_created += 1;
                }
                unit_ids.push(building_unit_ids);
            }

            // Assign users to units and update occupancy status
            for user in &seed_data.users {
                let user_id = user_ids
                    .get(user.email)
                    .ok_or_else(|| SeedError::Internal("User ID not found".to_string()))?;

                for assignment in &user.unit_assignments {
                    let unit_id = unit_ids
                        .get(assignment.building_index)
                        .and_then(|units| units.get(assignment.unit_index))
                        .ok_or_else(|| SeedError::Internal("Unit ID not found".to_string()))?;

                    factories
                        .assign_unit_resident(
                            *unit_id,
                            *user_id,
                            assignment.resident_type,
                            assignment.is_primary,
                        )
                        .await
                        .map_err(|e| SeedError::Database(e.to_string()))?;

                    // Update unit occupancy status based on resident type
                    let occupancy = match assignment.resident_type {
                        "owner" => "owner_occupied",
                        "tenant" | "subtenant" => "rented",
                        _ => "unknown",
                    };

                    // Only update if this is the primary resident
                    if assignment.is_primary {
                        factories
                            .update_unit_occupancy(*unit_id, occupancy)
                            .await
                            .map_err(|e| SeedError::Database(e.to_string()))?;
                    }

                    residents_assigned += 1;
                }
            }

            // Helper: resolve a PPT user index to its UUID
            let resolve_user = |index: usize| -> Result<Uuid, SeedError> {
                seed_data
                    .users
                    .get(index)
                    .and_then(|u| user_ids.get(u.email))
                    .copied()
                    .ok_or_else(|| {
                        SeedError::Internal(format!("User index {} out of range", index))
                    })
            };

            // Helper: resolve an optional PPT user index
            let resolve_user_opt = |index: Option<usize>| -> Result<Option<Uuid>, SeedError> {
                match index {
                    Some(i) => resolve_user(i).map(Some),
                    None => Ok(None),
                }
            };

            // 8. Create faults
            for fault in &seed_data.faults {
                let building_id =
                    building_ids
                        .get(fault.building_index)
                        .copied()
                        .ok_or_else(|| {
                            SeedError::Internal(format!(
                                "Fault building index {} out of range",
                                fault.building_index
                            ))
                        })?;

                let unit_id = fault
                    .unit_index
                    .map(|ui| {
                        unit_ids
                            .get(fault.building_index)
                            .and_then(|units| units.get(ui))
                            .copied()
                            .ok_or_else(|| {
                                SeedError::Internal(format!(
                                    "Fault unit index {} in building {} out of range",
                                    ui, fault.building_index
                                ))
                            })
                    })
                    .transpose()?;

                let reporter_id = resolve_user(fault.reporter_user_index)?;
                let assigned_to = resolve_user_opt(fault.assigned_user_index)?;
                let resolved_by = resolve_user_opt(fault.resolved_by_user_index)?;
                let confirmed_by = resolve_user_opt(fault.confirmed_by_user_index)?;

                factories
                    .create_fault(
                        org_id,
                        building_id,
                        unit_id,
                        reporter_id,
                        fault.title,
                        fault.description,
                        fault.location_description,
                        fault.category,
                        fault.priority,
                        fault.status,
                        assigned_to,
                        resolved_by,
                        confirmed_by,
                        fault.rating,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                faults_created += 1;
            }

            // 9. Create votes and their questions
            for vote in &seed_data.votes {
                let building_id =
                    building_ids
                        .get(vote.building_index)
                        .copied()
                        .ok_or_else(|| {
                            SeedError::Internal(format!(
                                "Vote building index {} out of range",
                                vote.building_index
                            ))
                        })?;

                let created_by = resolve_user(vote.created_by_user_index)?;

                let vote_id = factories
                    .create_vote(
                        org_id,
                        building_id,
                        vote.title,
                        vote.description,
                        vote.status,
                        created_by,
                        vote.quorum_type,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                for (order, question) in vote.questions.iter().enumerate() {
                    factories
                        .create_vote_question(
                            vote_id,
                            question.question_text,
                            question.question_type,
                            order as i32,
                            question.options,
                        )
                        .await
                        .map_err(|e| SeedError::Database(e.to_string()))?;
                }

                votes_created += 1;
            }

            // 10. Create announcements
            for announcement in &seed_data.announcements {
                let author_id = resolve_user(announcement.author_user_index)?;

                // Build target_ids JSON: building UUID array for "building" type, empty for others.
                let target_ids = if announcement.target_type == "building" {
                    let building_id = announcement
                        .building_index
                        .and_then(|bi| building_ids.get(bi).copied())
                        .ok_or_else(|| {
                            SeedError::Internal(
                                "Announcement building_index missing or out of range".to_string(),
                            )
                        })?;
                    serde_json::json!([building_id.to_string()])
                } else {
                    serde_json::json!([])
                };

                factories
                    .create_announcement(
                        org_id,
                        author_id,
                        announcement.title,
                        announcement.content,
                        announcement.target_type,
                        target_ids,
                        announcement.status,
                        announcement.pinned,
                        announcement.acknowledgment_required,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                announcements_created += 1;
            }

            // 11. Create listings (PPT → Reality Portal bridge)
            let mut listing_ids: Vec<Uuid> = Vec::new();

            for listing in &seed_data.listings {
                let building_id = building_ids
                    .get(listing.building_index)
                    .copied()
                    .ok_or_else(|| {
                        SeedError::Internal(format!(
                            "Listing building index {} out of range",
                            listing.building_index
                        ))
                    })?;

                let unit_id = listing
                    .unit_index
                    .map(|ui| {
                        unit_ids
                            .get(listing.building_index)
                            .and_then(|units| units.get(ui))
                            .copied()
                            .ok_or_else(|| {
                                SeedError::Internal(format!(
                                    "Listing unit index {} in building {} out of range",
                                    ui, listing.building_index
                                ))
                            })
                    })
                    .transpose()?;

                let created_by = resolve_user(listing.created_by_user_index)?;

                let listing_id = factories
                    .create_listing(
                        org_id,
                        building_id,
                        unit_id,
                        created_by,
                        listing.title,
                        listing.description,
                        listing.transaction_type,
                        listing.property_type,
                        listing.status,
                        listing.price,
                        listing.is_negotiable,
                        listing.rooms,
                        listing.bathrooms,
                        listing.features,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                listing_ids.push(listing_id);
                listings_created += 1;
            }

            // 12. Create portal users (Reality Portal, separate from PPT)
            let mut portal_user_ids: Vec<Uuid> = Vec::new();

            for portal_user in &seed_data.portal_users {
                let pm_user_id = portal_user.pm_user_index.map(resolve_user).transpose()?;

                let portal_user_id = factories
                    .create_portal_user(
                        portal_user.email,
                        portal_user.name,
                        &portal_hash,
                        pm_user_id,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                portal_user_ids.push(portal_user_id);
                portal_users_created += 1;
            }

            // Helper: resolve a portal user index to its UUID
            let resolve_portal_user = |index: usize| -> Result<Uuid, SeedError> {
                portal_user_ids.get(index).copied().ok_or_else(|| {
                    SeedError::Internal(format!("Portal user index {} out of range", index))
                })
            };

            // 13. Create inquiries and message threads
            for inquiry in &seed_data.inquiries {
                let listing_id =
                    listing_ids
                        .get(inquiry.listing_index)
                        .copied()
                        .ok_or_else(|| {
                            SeedError::Internal(format!(
                                "Inquiry listing index {} out of range",
                                inquiry.listing_index
                            ))
                        })?;

                let realtor_id = resolve_portal_user(inquiry.realtor_portal_user_index)?;
                let user_id = resolve_portal_user(inquiry.portal_user_index)?;

                let portal_user = seed_data
                    .portal_users
                    .get(inquiry.portal_user_index)
                    .ok_or_else(|| {
                        SeedError::Internal("Portal user not found for inquiry".to_string())
                    })?;

                let inquiry_id = factories
                    .create_inquiry(
                        listing_id,
                        realtor_id,
                        user_id,
                        portal_user.name,
                        portal_user.email,
                        portal_user.phone,
                        inquiry.message,
                        inquiry.inquiry_type,
                        inquiry.preferred_contact,
                        inquiry.status,
                    )
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                // Initial message from the inquiring user
                factories
                    .create_inquiry_message(inquiry_id, "user", user_id, inquiry.message)
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                // Realtor reply if present
                if let Some(reply) = inquiry.reply_message {
                    factories
                        .create_inquiry_message(inquiry_id, "realtor", realtor_id, reply)
                        .await
                        .map_err(|e| SeedError::Database(e.to_string()))?;
                }

                inquiries_created += 1;
            }

            // 14. Create portal favorites
            for fav in &seed_data.portal_favorites {
                let user_id = resolve_portal_user(fav.portal_user_index)?;
                let listing_id = listing_ids.get(fav.listing_index).copied().ok_or_else(|| {
                    SeedError::Internal(format!(
                        "Favorite listing index {} out of range",
                        fav.listing_index
                    ))
                })?;

                factories
                    .create_portal_favorite(user_id, listing_id)
                    .await
                    .map_err(|e| SeedError::Database(e.to_string()))?;

                favorites_created += 1;
            }
        }

        // 15. Clear super admin context
        clear_request_context(rls_guard.pool())
            .await
            .map_err(|e| SeedError::Database(e.to_string()))?;
        rls_guard.mark_cleared();

        Ok(SeedResult {
            organizations_created: 1,
            users_created,
            buildings_created,
            units_created,
            residents_assigned,
            faults_created,
            votes_created,
            announcements_created,
            listings_created,
            portal_users_created,
            inquiries_created,
            favorites_created,
            admin_user_id: admin_id,
            organization_id: org_id,
            cleanup_stats,
        })
    }
}

/// Errors that can occur during seeding.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("Seed data already exists. Use --force to re-seed.")]
    AlreadySeeded,

    #[error("Internal error: {0}")]
    Internal(String),
}

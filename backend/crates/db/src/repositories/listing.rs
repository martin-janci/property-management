//! Listing repository (Epic 15: Property Listings & Multi-Portal Sync).
//! Extended for Epic 105: Portal Syndication.

use crate::models::listing::{
    listing_status, syndication_status, CreateListing, CreateListingPhoto, CreateSyndication,
    Listing, ListingListQuery, ListingPhoto, ListingStatistics, ListingSummary, ListingSyndication,
    OrganizationSyndicationStats, PortalStats, PortalSyndicationStatus, PortalWebhookEvent,
    PropertyTypeCount, SyndicationDashboardQuery, SyndicationHealthStatus,
    SyndicationStatusDashboard, UpdateListing, UpdateListingStatus,
};
use crate::DbPool;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, FromRow};
use uuid::Uuid;

/// Row for listing summary with first photo.
#[derive(Debug, FromRow)]
struct ListingSummaryRow {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub title: String,
    pub property_type: String,
    pub transaction_type: String,
    pub price: Decimal,
    pub currency: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub city: String,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub published_at: Option<chrono::DateTime<Utc>>,
    pub photo_url: Option<String>,
}

/// Repository for listing operations.
#[derive(Clone)]
pub struct ListingRepository {
    pool: DbPool,
}

impl ListingRepository {
    /// Create a new ListingRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Listing CRUD
    // ========================================================================

    /// Create a new listing (Story 15.1).
    pub async fn create(
        &self,
        data: CreateListing,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Listing, SqlxError> {
        let features_json = serde_json::to_value(&data.features).unwrap_or(serde_json::json!([]));

        let listing = sqlx::query_as::<_, Listing>(
            r#"
            INSERT INTO listings (
                organization_id, unit_id, created_by, status, transaction_type,
                title, description, property_type, size_sqm, rooms, bathrooms,
                floor, total_floors, street, city, postal_code, country,
                latitude, longitude, price, currency, is_negotiable, features
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(user_id)
        .bind(listing_status::DRAFT)
        .bind(&data.transaction_type)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.property_type)
        .bind(data.size_sqm)
        .bind(data.rooms)
        .bind(data.bathrooms)
        .bind(data.floor)
        .bind(data.total_floors)
        .bind(&data.street)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(data.latitude)
        .bind(data.longitude)
        .bind(data.price)
        .bind(&data.currency)
        .bind(data.is_negotiable)
        .bind(&features_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Find listing by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Listing>, SqlxError> {
        let listing = sqlx::query_as::<_, Listing>(r#"SELECT * FROM listings WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(listing)
    }

    /// Find listing by ID and organization (with auth check).
    pub async fn find_by_id_and_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Listing>, SqlxError> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"SELECT * FROM listings WHERE id = $1 AND organization_id = $2"#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(listing)
    }

    /// List listings for an organization with filtering.
    pub async fn list(
        &self,
        org_id: Uuid,
        query: &ListingListQuery,
    ) -> Result<Vec<ListingSummary>, SqlxError> {
        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        let rows = sqlx::query_as::<_, ListingSummaryRow>(
            r#"
            SELECT
                l.id, l.organization_id, l.title, l.property_type, l.transaction_type,
                l.price, l.currency, l.size_sqm, l.rooms, l.city, l.status,
                l.created_at, l.published_at,
                (SELECT url FROM listing_photos WHERE listing_id = l.id ORDER BY display_order LIMIT 1) as photo_url
            FROM listings l
            WHERE l.organization_id = $1
                AND ($2::text IS NULL OR l.status = $2)
                AND ($3::text IS NULL OR l.transaction_type = $3)
                AND ($4::text IS NULL OR l.property_type = $4)
                AND ($5::text IS NULL OR l.city ILIKE '%' || $5 || '%')
                AND ($6::numeric IS NULL OR l.price >= $6)
                AND ($7::numeric IS NULL OR l.price <= $7)
                AND ($8::int IS NULL OR l.rooms >= $8)
                AND ($9::int IS NULL OR l.rooms <= $9)
            ORDER BY l.created_at DESC
            LIMIT $10 OFFSET $11
            "#,
        )
        .bind(org_id)
        .bind(&query.status)
        .bind(&query.transaction_type)
        .bind(&query.property_type)
        .bind(&query.city)
        .bind(query.price_min)
        .bind(query.price_max)
        .bind(query.rooms_min)
        .bind(query.rooms_max)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        // Map to ListingSummary
        let summaries = rows
            .into_iter()
            .map(|r| ListingSummary {
                id: r.id,
                organization_id: r.organization_id,
                title: r.title,
                property_type: r.property_type,
                transaction_type: r.transaction_type,
                price: r.price,
                currency: r.currency,
                size_sqm: r.size_sqm,
                rooms: r.rooms,
                city: r.city,
                status: r.status,
                created_at: r.created_at,
                published_at: r.published_at,
                photo_url: r.photo_url,
            })
            .collect();

        Ok(summaries)
    }

    /// Count listings for an organization.
    pub async fn count(&self, org_id: Uuid, query: &ListingListQuery) -> Result<i64, SqlxError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM listings l
            WHERE l.organization_id = $1
                AND ($2::text IS NULL OR l.status = $2)
                AND ($3::text IS NULL OR l.transaction_type = $3)
                AND ($4::text IS NULL OR l.property_type = $4)
                AND ($5::text IS NULL OR l.city ILIKE '%' || $5 || '%')
                AND ($6::numeric IS NULL OR l.price >= $6)
                AND ($7::numeric IS NULL OR l.price <= $7)
                AND ($8::int IS NULL OR l.rooms >= $8)
                AND ($9::int IS NULL OR l.rooms <= $9)
            "#,
        )
        .bind(org_id)
        .bind(&query.status)
        .bind(&query.transaction_type)
        .bind(&query.property_type)
        .bind(&query.city)
        .bind(query.price_min)
        .bind(query.price_max)
        .bind(query.rooms_min)
        .bind(query.rooms_max)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Update a listing.
    pub async fn update(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: UpdateListing,
    ) -> Result<Listing, SqlxError> {
        let features_json = data
            .features
            .map(|f| serde_json::to_value(f).unwrap_or(serde_json::json!([])));

        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings SET
                title = COALESCE($3, title),
                description = COALESCE($4, description),
                property_type = COALESCE($5, property_type),
                size_sqm = COALESCE($6, size_sqm),
                rooms = COALESCE($7, rooms),
                bathrooms = COALESCE($8, bathrooms),
                floor = COALESCE($9, floor),
                total_floors = COALESCE($10, total_floors),
                street = COALESCE($11, street),
                city = COALESCE($12, city),
                postal_code = COALESCE($13, postal_code),
                country = COALESCE($14, country),
                latitude = COALESCE($15, latitude),
                longitude = COALESCE($16, longitude),
                price = COALESCE($17, price),
                currency = COALESCE($18, currency),
                is_negotiable = COALESCE($19, is_negotiable),
                features = COALESCE($20, features),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.property_type)
        .bind(data.size_sqm)
        .bind(data.rooms)
        .bind(data.bathrooms)
        .bind(data.floor)
        .bind(data.total_floors)
        .bind(&data.street)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(data.latitude)
        .bind(data.longitude)
        .bind(data.price)
        .bind(&data.currency)
        .bind(data.is_negotiable)
        .bind(&features_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Update listing status (Story 15.4).
    pub async fn update_status(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: UpdateListingStatus,
    ) -> Result<Listing, SqlxError> {
        // Determine additional updates based on status
        let (published_at, sold_at) = match data.status.as_str() {
            listing_status::ACTIVE => (Some(Utc::now()), None::<chrono::DateTime<Utc>>),
            listing_status::SOLD | listing_status::RENTED => (None, Some(Utc::now())),
            _ => (None, None),
        };

        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings SET
                status = $3,
                published_at = COALESCE($4, published_at),
                sold_at = COALESCE($5, sold_at),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.status)
        .bind(published_at)
        .bind(sold_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Delete a listing.
    pub async fn delete(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM listings WHERE id = $1 AND organization_id = $2"#)
            .bind(id)
            .bind(org_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Phase 4: Global Portal Publish State (is_published)
    // ========================================================================
    //
    // These methods control whether a listing appears in the platform-wide
    // reality portal (4th RLS context). They are orthogonal to the existing
    // workflow `status` and to the syndication-to-external-portals flow
    // (Epic 105). The pattern is binary first per ROADMAP.md Phase 4; a
    // future migration can evolve `is_published` into a channel set without
    // breaking this API.

    /// Mark a listing as published to the global reality portal.
    /// Sets `is_published = TRUE` and (re)stamps `published_at = NOW()`.
    /// Org-scoped — only the owning agency can flip its own listings.
    pub async fn set_published(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Listing>, SqlxError> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings SET
                is_published = TRUE,
                published_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Withdraw a listing from the global reality portal.
    /// Sets `is_published = FALSE` and clears `published_at`.
    /// The listing row itself is preserved (invariant I-D — a listing exists
    /// once; un-publishing only flips the global-read OR-clause off).
    pub async fn set_unpublished(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Listing>, SqlxError> {
        let listing = sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings SET
                is_published = FALSE,
                published_at = NULL,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Get listing statistics for organization.
    pub async fn get_statistics(&self, org_id: Uuid) -> Result<ListingStatistics, SqlxError> {
        let total_listings: (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) FROM listings WHERE organization_id = $1"#)
                .bind(org_id)
                .fetch_one(&self.pool)
                .await?;

        let active_listings: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM listings WHERE organization_id = $1 AND status = $2"#,
        )
        .bind(org_id)
        .bind(listing_status::ACTIVE)
        .fetch_one(&self.pool)
        .await?;

        let draft_listings: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM listings WHERE organization_id = $1 AND status = $2"#,
        )
        .bind(org_id)
        .bind(listing_status::DRAFT)
        .fetch_one(&self.pool)
        .await?;

        let sold_listings: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM listings WHERE organization_id = $1 AND status = $2"#,
        )
        .bind(org_id)
        .bind(listing_status::SOLD)
        .fetch_one(&self.pool)
        .await?;

        let rented_listings: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM listings WHERE organization_id = $1 AND status = $2"#,
        )
        .bind(org_id)
        .bind(listing_status::RENTED)
        .fetch_one(&self.pool)
        .await?;

        let by_property_type: Vec<PropertyTypeCount> = sqlx::query_as(
            r#"
            SELECT property_type, COUNT(*) as count
            FROM listings
            WHERE organization_id = $1
            GROUP BY property_type
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ListingStatistics {
            total_listings: total_listings.0,
            active_listings: active_listings.0,
            draft_listings: draft_listings.0,
            sold_listings: sold_listings.0,
            rented_listings: rented_listings.0,
            by_property_type,
        })
    }

    // ========================================================================
    // Listing Photos (Story 15.2)
    // ========================================================================

    /// Add a photo to a listing.
    pub async fn add_photo(
        &self,
        listing_id: Uuid,
        data: CreateListingPhoto,
    ) -> Result<ListingPhoto, SqlxError> {
        // Get next display order if not provided
        let display_order = match data.display_order {
            Some(order) => order,
            None => {
                let row: (i32,) = sqlx::query_as(
                    r#"SELECT COALESCE(MAX(display_order), 0) + 1 FROM listing_photos WHERE listing_id = $1"#,
                )
                .bind(listing_id)
                .fetch_one(&self.pool)
                .await?;
                row.0
            }
        };

        let photo = sqlx::query_as::<_, ListingPhoto>(
            r#"
            INSERT INTO listing_photos (listing_id, url, thumbnail_url, medium_url, display_order, alt_text)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(&data.url)
        .bind(&data.thumbnail_url)
        .bind(&data.medium_url)
        .bind(display_order)
        .bind(&data.alt_text)
        .fetch_one(&self.pool)
        .await?;

        Ok(photo)
    }

    /// Get photos for a listing.
    pub async fn get_photos(&self, listing_id: Uuid) -> Result<Vec<ListingPhoto>, SqlxError> {
        let photos = sqlx::query_as::<_, ListingPhoto>(
            r#"SELECT * FROM listing_photos WHERE listing_id = $1 ORDER BY display_order"#,
        )
        .bind(listing_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(photos)
    }

    /// Delete a photo.
    pub async fn delete_photo(&self, photo_id: Uuid, listing_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM listing_photos WHERE id = $1 AND listing_id = $2"#)
            .bind(photo_id)
            .bind(listing_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Reorder photos for a listing.
    pub async fn reorder_photos(
        &self,
        listing_id: Uuid,
        photo_ids: Vec<Uuid>,
    ) -> Result<(), SqlxError> {
        for (order, photo_id) in photo_ids.iter().enumerate() {
            sqlx::query(
                r#"UPDATE listing_photos SET display_order = $1 WHERE id = $2 AND listing_id = $3"#,
            )
            .bind(order as i32)
            .bind(photo_id)
            .bind(listing_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    // ========================================================================
    // Listing Syndication (Story 15.3)
    // ========================================================================

    /// Create syndications for a listing.
    pub async fn create_syndications(
        &self,
        listing_id: Uuid,
        data: CreateSyndication,
    ) -> Result<Vec<ListingSyndication>, SqlxError> {
        let mut syndications = Vec::new();

        for portal in data.portals {
            let syndication = sqlx::query_as::<_, ListingSyndication>(
                r#"
                INSERT INTO listing_syndications (listing_id, portal, status)
                VALUES ($1, $2, $3)
                ON CONFLICT (listing_id, portal) DO UPDATE SET
                    status = $3,
                    updated_at = NOW()
                RETURNING *
                "#,
            )
            .bind(listing_id)
            .bind(&portal)
            .bind(syndication_status::PENDING)
            .fetch_one(&self.pool)
            .await?;

            syndications.push(syndication);
        }

        Ok(syndications)
    }

    /// Get syndications for a listing.
    pub async fn get_syndications(
        &self,
        listing_id: Uuid,
    ) -> Result<Vec<ListingSyndication>, SqlxError> {
        let syndications = sqlx::query_as::<_, ListingSyndication>(
            r#"SELECT * FROM listing_syndications WHERE listing_id = $1 ORDER BY portal"#,
        )
        .bind(listing_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(syndications)
    }

    /// Update syndication status after sync.
    pub async fn update_syndication_status(
        &self,
        listing_id: Uuid,
        portal: &str,
        status: &str,
        external_id: Option<&str>,
        error: Option<&str>,
    ) -> Result<ListingSyndication, SqlxError> {
        let synced_at = if status == syndication_status::SYNCED {
            Some(Utc::now())
        } else {
            None
        };

        let syndication = sqlx::query_as::<_, ListingSyndication>(
            r#"
            UPDATE listing_syndications SET
                status = $3,
                external_id = COALESCE($4, external_id),
                last_error = $5,
                synced_at = COALESCE($6, synced_at),
                updated_at = NOW()
            WHERE listing_id = $1 AND portal = $2
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(portal)
        .bind(status)
        .bind(external_id)
        .bind(error)
        .bind(synced_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(syndication)
    }

    /// Remove syndication for a listing from a portal.
    pub async fn remove_syndication(
        &self,
        listing_id: Uuid,
        portal: &str,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"DELETE FROM listing_syndications WHERE listing_id = $1 AND portal = $2"#,
        )
        .bind(listing_id)
        .bind(portal)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Find listings that need syncing to a portal.
    pub async fn find_pending_syndications(
        &self,
        portal: &str,
    ) -> Result<Vec<ListingSyndication>, SqlxError> {
        let syndications = sqlx::query_as::<_, ListingSyndication>(
            r#"
            SELECT ls.* FROM listing_syndications ls
            JOIN listings l ON l.id = ls.listing_id
            WHERE ls.portal = $1
              AND ls.status = $2
              AND l.status = $3
            ORDER BY ls.created_at
            LIMIT 100
            "#,
        )
        .bind(portal)
        .bind(syndication_status::PENDING)
        .bind(listing_status::ACTIVE)
        .fetch_all(&self.pool)
        .await?;

        Ok(syndications)
    }

    // ========================================================================
    // Epic 105: Portal Syndication
    // ========================================================================

    /// Get syndication status dashboard for a listing.
    pub async fn get_syndication_status_dashboard(
        &self,
        listing_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<SyndicationStatusDashboard>, SqlxError> {
        // First get the listing
        let listing = match self.find_by_id_and_org(listing_id, org_id).await? {
            Some(l) => l,
            None => return Ok(None),
        };

        // Get all syndications for this listing with their stats
        let syndications = self.get_syndications(listing_id).await?;

        // Get portal-specific stats (views, inquiries)
        let portal_statuses: Vec<PortalSyndicationStatus> = syndications
            .into_iter()
            .map(|s| {
                // Get stats from webhook events (views/inquiries would be tracked there)
                PortalSyndicationStatus {
                    portal: s.portal,
                    status: s.status,
                    external_id: s.external_id,
                    synced_at: s.synced_at,
                    last_error: s.last_error,
                    views: 0,     // Would be populated from webhook events
                    inquiries: 0, // Would be populated from webhook events
                    last_activity_at: s.synced_at,
                }
            })
            .collect();

        // Determine overall health status
        let overall_status = if portal_statuses.is_empty() {
            SyndicationHealthStatus::NotConfigured
        } else {
            let synced_count = portal_statuses
                .iter()
                .filter(|s| s.status == syndication_status::SYNCED)
                .count();
            let failed_count = portal_statuses
                .iter()
                .filter(|s| s.status == syndication_status::FAILED)
                .count();

            if failed_count == portal_statuses.len() {
                SyndicationHealthStatus::Unhealthy
            } else if failed_count > 0 {
                SyndicationHealthStatus::Degraded
            } else if synced_count == portal_statuses.len() {
                SyndicationHealthStatus::Healthy
            } else {
                SyndicationHealthStatus::Degraded
            }
        };

        let total_views: i64 = portal_statuses.iter().map(|s| s.views).sum();
        let total_inquiries: i64 = portal_statuses.iter().map(|s| s.inquiries).sum();

        Ok(Some(SyndicationStatusDashboard {
            listing_id: listing.id,
            listing_title: listing.title,
            listing_status: listing.status,
            portal_statuses,
            overall_status,
            total_views,
            total_inquiries,
        }))
    }

    /// Get organization-wide syndication statistics.
    pub async fn get_organization_syndication_stats(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSyndicationStats, SqlxError> {
        // Get counts by status
        let total_active: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM listing_syndications ls
            JOIN listings l ON l.id = ls.listing_id
            WHERE l.organization_id = $1 AND ls.status = $2
            "#,
        )
        .bind(org_id)
        .bind(syndication_status::SYNCED)
        .fetch_one(&self.pool)
        .await?;

        let total_pending: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM listing_syndications ls
            JOIN listings l ON l.id = ls.listing_id
            WHERE l.organization_id = $1 AND ls.status = $2
            "#,
        )
        .bind(org_id)
        .bind(syndication_status::PENDING)
        .fetch_one(&self.pool)
        .await?;

        let total_failed: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM listing_syndications ls
            JOIN listings l ON l.id = ls.listing_id
            WHERE l.organization_id = $1 AND ls.status = $2
            "#,
        )
        .bind(org_id)
        .bind(syndication_status::FAILED)
        .fetch_one(&self.pool)
        .await?;

        // Get stats by portal
        let by_portal = sqlx::query_as::<_, PortalStatsRow>(
            r#"
            SELECT
                ls.portal,
                COUNT(*) FILTER (WHERE ls.status = 'synced') as active_count,
                COUNT(*) FILTER (WHERE ls.status = 'pending') as pending_count,
                COUNT(*) FILTER (WHERE ls.status = 'failed') as failed_count
            FROM listing_syndications ls
            JOIN listings l ON l.id = ls.listing_id
            WHERE l.organization_id = $1
            GROUP BY ls.portal
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| PortalStats {
            portal: r.portal,
            active_count: r.active_count,
            pending_count: r.pending_count,
            failed_count: r.failed_count,
            views: 0,     // Would come from webhook events
            inquiries: 0, // Would come from webhook events
        })
        .collect();

        Ok(OrganizationSyndicationStats {
            total_active: total_active.0,
            total_pending: total_pending.0,
            total_failed: total_failed.0,
            total_views: 0,     // Would come from aggregated webhook events
            total_inquiries: 0, // Would come from aggregated webhook events
            by_portal,
        })
    }

    /// Get syndication dashboard with pagination.
    pub async fn get_syndication_dashboard(
        &self,
        org_id: Uuid,
        query: &SyndicationDashboardQuery,
    ) -> Result<(Vec<SyndicationStatusDashboard>, i64), SqlxError> {
        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        // Get listings with syndications
        let rows = sqlx::query_as::<_, ListingWithSyndicationRow>(
            r#"
            SELECT DISTINCT ON (l.id)
                l.id as listing_id,
                l.title as listing_title,
                l.status as listing_status,
                l.created_at
            FROM listings l
            JOIN listing_syndications ls ON ls.listing_id = l.id
            WHERE l.organization_id = $1
                AND ($2::text IS NULL OR ls.portal = $2)
                AND ($3::text IS NULL OR ls.status = $3)
            ORDER BY l.id, l.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(&query.portal)
        .bind(&query.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        // Count total
        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT l.id)
            FROM listings l
            JOIN listing_syndications ls ON ls.listing_id = l.id
            WHERE l.organization_id = $1
                AND ($2::text IS NULL OR ls.portal = $2)
                AND ($3::text IS NULL OR ls.status = $3)
            "#,
        )
        .bind(org_id)
        .bind(&query.portal)
        .bind(&query.status)
        .fetch_one(&self.pool)
        .await?;

        // Build dashboard entries
        let mut dashboards = Vec::new();
        for row in rows {
            if let Some(dashboard) = self
                .get_syndication_status_dashboard(row.listing_id, org_id)
                .await?
            {
                dashboards.push(dashboard);
            }
        }

        Ok((dashboards, total.0))
    }

    /// Get syndications for a listing that need status propagation.
    pub async fn get_syndications_for_status_propagation(
        &self,
        listing_id: Uuid,
    ) -> Result<Vec<ListingSyndication>, SqlxError> {
        let syndications = sqlx::query_as::<_, ListingSyndication>(
            r#"
            SELECT * FROM listing_syndications
            WHERE listing_id = $1
              AND status IN ($2, $3)
            ORDER BY portal
            "#,
        )
        .bind(listing_id)
        .bind(syndication_status::SYNCED)
        .bind(syndication_status::PENDING)
        .fetch_all(&self.pool)
        .await?;

        Ok(syndications)
    }

    /// Find syndication by external ID and portal.
    pub async fn find_syndication_by_external_id(
        &self,
        portal: &str,
        external_id: &str,
    ) -> Result<Option<ListingSyndication>, SqlxError> {
        let syndication = sqlx::query_as::<_, ListingSyndication>(
            r#"
            SELECT * FROM listing_syndications
            WHERE portal = $1 AND external_id = $2
            "#,
        )
        .bind(portal)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(syndication)
    }

    /// Record a portal webhook event.
    ///
    /// Idempotent on `(portal, event_type, external_id)` when `external_id` is
    /// present (the dedup net from migration 00219, #2358): a redelivery of an
    /// already-recorded event hits the partial unique index and the
    /// `ON CONFLICT ... DO NOTHING` clause suppresses the insert.
    ///
    /// Returns:
    /// - `Ok(Some(event))` — a new row was inserted.
    /// - `Ok(None)` — the event already existed (duplicate delivery); no row
    ///   was written. Callers on the lead-bearing inquiry path treat this as an
    ///   idempotent success so retries of an already-recorded lead don't loop.
    /// - `Err(_)` — a genuine write failure; the caller must not ack.
    pub async fn record_webhook_event(
        &self,
        listing_id: Uuid,
        syndication_id: Uuid,
        portal: &str,
        event_type: &str,
        external_id: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<Option<PortalWebhookEvent>, SqlxError> {
        let event = sqlx::query_as::<_, PortalWebhookEvent>(
            r#"
            INSERT INTO portal_webhook_events
                (listing_id, syndication_id, portal, event_type, external_id, payload)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (portal, event_type, external_id) WHERE external_id IS NOT NULL
            DO NOTHING
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(syndication_id)
        .bind(portal)
        .bind(event_type)
        .bind(external_id)
        .bind(payload)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    /// Mark webhook event as processed.
    pub async fn mark_webhook_event_processed(
        &self,
        event_id: Uuid,
    ) -> Result<Option<PortalWebhookEvent>, SqlxError> {
        let event = sqlx::query_as::<_, PortalWebhookEvent>(
            r#"
            UPDATE portal_webhook_events
            SET processed = true, processed_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    /// Get unprocessed webhook events for a portal.
    pub async fn get_unprocessed_webhook_events(
        &self,
        portal: &str,
        limit: i32,
    ) -> Result<Vec<PortalWebhookEvent>, SqlxError> {
        let events = sqlx::query_as::<_, PortalWebhookEvent>(
            r#"
            SELECT * FROM portal_webhook_events
            WHERE portal = $1 AND processed = false
            ORDER BY created_at
            LIMIT $2
            "#,
        )
        .bind(portal)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Update syndication with views and inquiries count.
    pub async fn increment_syndication_stats(
        &self,
        syndication_id: Uuid,
        views_delta: i64,
        inquiries_delta: i64,
    ) -> Result<(), SqlxError> {
        // This would update a stats table if we had one, for now we just record the event
        // The actual counting would be done by aggregating portal_webhook_events
        sqlx::query(
            r#"
            UPDATE listing_syndications
            SET updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(syndication_id)
        .execute(&self.pool)
        .await?;

        // Log the stat update (views_delta and inquiries_delta are tracked via webhook events)
        let _ = views_delta;
        let _ = inquiries_delta;

        Ok(())
    }
}

/// Row for portal stats query.
#[derive(Debug, FromRow)]
struct PortalStatsRow {
    portal: String,
    active_count: i64,
    pending_count: i64,
    failed_count: i64,
}

/// Row for listing with syndication.
#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ListingWithSyndicationRow {
    listing_id: Uuid,
    listing_title: String,
    listing_status: String,
    created_at: chrono::DateTime<Utc>,
}

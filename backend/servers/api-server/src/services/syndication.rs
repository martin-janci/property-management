//! Syndication service (Epic 105: Portal Syndication).
//!
//! Story 105.1: Syndication Job Queue
//! Story 105.2: Status Change Propagation

use db::models::{
    listing_portal, listing_status, syndication_job_type, syndication_status, CreateBackgroundJob,
    Listing, ListingSyndication, SyndicationJobPayload,
};
use db::repositories::{BackgroundJobRepository, ListingRepository};
use serde_json::json;
use uuid::Uuid;

/// Syndication queue name for background jobs.
pub const SYNDICATION_QUEUE: &str = "syndication";

/// Worker identifier recorded on claimed syndication jobs.
const SYNDICATION_WORKER_ID: &str = "api-server-syndication-drain";

/// Configuration for the syndication queue drain worker.
#[derive(Debug, Clone)]
pub struct SyndicationDrainConfig {
    /// When false, the worker does nothing.
    pub enabled: bool,
    /// How often to poll the queue.
    pub poll_interval_secs: u64,
    /// Max jobs processed per tick.
    pub batch_limit: usize,
}

impl Default for SyndicationDrainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 30,
            batch_limit: 50,
        }
    }
}

impl SyndicationDrainConfig {
    /// Build from `SYNDICATION_DRAIN_ENABLED` / `SYNDICATION_DRAIN_INTERVAL_SECS`.
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            enabled: std::env::var("SYNDICATION_DRAIN_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(d.enabled),
            poll_interval_secs: std::env::var("SYNDICATION_DRAIN_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|&n| n > 0)
                .unwrap_or(d.poll_interval_secs),
            ..d
        }
    }
}

/// Spawn the background task that periodically drains the syndication queue.
pub fn spawn_drain_worker(
    service: SyndicationService,
    config: SyndicationDrainConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !config.enabled {
            tracing::info!("Syndication drain worker disabled - not starting");
            return;
        }
        tracing::info!(
            poll_interval_secs = config.poll_interval_secs,
            "Syndication drain worker started"
        );
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_secs(config.poll_interval_secs));
        loop {
            ticker.tick().await;
            match service.drain_queue(config.batch_limit).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(processed = n, "Drained syndication jobs"),
                Err(e) => tracing::error!(error = %e, "Syndication drain tick failed"),
            }
        }
    })
}

/// Portal status mapping from internal status to portal-specific status.
pub struct PortalStatusMapping {
    /// Internal status
    pub internal_status: &'static str,
    /// Mapped status for Reality Portal
    pub reality_portal: &'static str,
    /// Mapped status for SReality
    pub sreality: &'static str,
    /// Mapped status for Bezrealitky
    pub bezrealitky: &'static str,
    /// Mapped status for Nehnutelnosti.sk
    pub nehnutelnosti: &'static str,
}

/// Get portal-specific status mapping.
pub fn get_status_mapping(internal_status: &str) -> Option<PortalStatusMapping> {
    match internal_status {
        listing_status::ACTIVE => Some(PortalStatusMapping {
            internal_status: listing_status::ACTIVE,
            reality_portal: "active",
            sreality: "published",
            bezrealitky: "visible",
            nehnutelnosti: "online",
        }),
        listing_status::PAUSED => Some(PortalStatusMapping {
            internal_status: listing_status::PAUSED,
            reality_portal: "paused",
            sreality: "hidden",
            bezrealitky: "hidden",
            nehnutelnosti: "offline",
        }),
        listing_status::SOLD => Some(PortalStatusMapping {
            internal_status: listing_status::SOLD,
            reality_portal: "sold",
            sreality: "sold",
            bezrealitky: "sold",
            nehnutelnosti: "sold",
        }),
        listing_status::RENTED => Some(PortalStatusMapping {
            internal_status: listing_status::RENTED,
            reality_portal: "rented",
            sreality: "rented",
            bezrealitky: "rented",
            nehnutelnosti: "rented",
        }),
        listing_status::ARCHIVED => Some(PortalStatusMapping {
            internal_status: listing_status::ARCHIVED,
            reality_portal: "archived",
            sreality: "removed",
            bezrealitky: "removed",
            nehnutelnosti: "removed",
        }),
        _ => None,
    }
}

/// Get the portal-specific status for a given portal.
pub fn get_portal_status(internal_status: &str, portal: &str) -> Option<&'static str> {
    let mapping = get_status_mapping(internal_status)?;

    match portal {
        listing_portal::REALITY_PORTAL => Some(mapping.reality_portal),
        listing_portal::SREALITY => Some(mapping.sreality),
        listing_portal::BEZREALITKY => Some(mapping.bezrealitky),
        listing_portal::NEHNUTELNOSTI => Some(mapping.nehnutelnosti),
        _ => None,
    }
}

/// Syndication service for managing portal syndication jobs.
#[derive(Clone)]
pub struct SyndicationService {
    background_job_repo: BackgroundJobRepository,
    listing_repo: ListingRepository,
}

impl SyndicationService {
    /// Create a new SyndicationService.
    pub fn new(
        background_job_repo: BackgroundJobRepository,
        listing_repo: ListingRepository,
    ) -> Self {
        Self {
            background_job_repo,
            listing_repo,
        }
    }

    /// Create syndication jobs for publishing a listing to specified portals (Story 105.1).
    ///
    /// Creates one background job per portal for async processing.
    pub async fn create_publish_jobs(
        &self,
        listing: &Listing,
        portals: &[String],
        created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, String> {
        let mut job_ids = Vec::new();

        for portal in portals {
            let payload = SyndicationJobPayload {
                listing_id: listing.id,
                portal: portal.clone(),
                operation: syndication_job_type::PUBLISH.to_string(),
                previous_status: None,
                new_status: Some(listing.status.clone()),
                organization_id: listing.organization_id,
            };

            let job_data = CreateBackgroundJob {
                job_type: syndication_job_type::PUBLISH.to_string(),
                priority: Some(5), // Normal priority for publish
                payload: serde_json::to_value(&payload).map_err(|e| e.to_string())?,
                scheduled_at: None, // Run immediately
                queue: Some(SYNDICATION_QUEUE.to_string()),
                max_attempts: Some(3),
                org_id: listing.organization_id,
            };

            let job = self
                .background_job_repo
                .create(job_data, created_by)
                .await
                .map_err(|e| format!("Failed to create job for portal {}: {}", portal, e))?;

            job_ids.push(job.id);

            tracing::info!(
                listing_id = %listing.id,
                portal = %portal,
                job_id = %job.id,
                "Created syndication publish job"
            );
        }

        Ok(job_ids)
    }

    /// Create status change propagation jobs (Story 105.2).
    ///
    /// When a listing status changes, propagate the change to all syndicated portals.
    pub async fn create_status_change_jobs(
        &self,
        listing: &Listing,
        previous_status: &str,
        new_status: &str,
        created_by: Option<Uuid>,
    ) -> Result<Vec<Uuid>, String> {
        // Get all active syndications for this listing
        let syndications = self
            .listing_repo
            .get_syndications_for_status_propagation(listing.id)
            .await
            .map_err(|e| format!("Failed to get syndications: {}", e))?;

        let mut job_ids = Vec::new();

        for syndication in syndications {
            // Determine the operation based on status change
            let operation = if new_status == listing_status::ARCHIVED
                || new_status == listing_status::SOLD
                || new_status == listing_status::RENTED
            {
                syndication_job_type::REMOVE
            } else {
                syndication_job_type::STATUS_CHANGE
            };

            let payload = SyndicationJobPayload {
                listing_id: listing.id,
                portal: syndication.portal.clone(),
                operation: operation.to_string(),
                previous_status: Some(previous_status.to_string()),
                new_status: Some(new_status.to_string()),
                organization_id: listing.organization_id,
            };

            // Higher priority for status changes (they should be processed quickly)
            let priority = match new_status {
                listing_status::SOLD | listing_status::RENTED => 10, // Highest
                listing_status::ARCHIVED => 8,
                listing_status::PAUSED => 6,
                _ => 5,
            };

            let job_data = CreateBackgroundJob {
                job_type: operation.to_string(),
                priority: Some(priority),
                payload: serde_json::to_value(&payload).map_err(|e| e.to_string())?,
                scheduled_at: None,
                queue: Some(SYNDICATION_QUEUE.to_string()),
                max_attempts: Some(3),
                org_id: listing.organization_id,
            };

            let job = self
                .background_job_repo
                .create(job_data, created_by)
                .await
                .map_err(|e| {
                    format!(
                        "Failed to create status change job for portal {}: {}",
                        syndication.portal, e
                    )
                })?;

            job_ids.push(job.id);

            tracing::info!(
                listing_id = %listing.id,
                portal = %syndication.portal,
                job_id = %job.id,
                previous_status = %previous_status,
                new_status = %new_status,
                "Created syndication status change job"
            );
        }

        Ok(job_ids)
    }

    /// Create an update job for a listing on a specific portal.
    pub async fn create_update_job(
        &self,
        listing: &Listing,
        syndication: &ListingSyndication,
        created_by: Option<Uuid>,
    ) -> Result<Uuid, String> {
        let payload = SyndicationJobPayload {
            listing_id: listing.id,
            portal: syndication.portal.clone(),
            operation: syndication_job_type::UPDATE.to_string(),
            previous_status: None,
            new_status: Some(listing.status.clone()),
            organization_id: listing.organization_id,
        };

        let job_data = CreateBackgroundJob {
            job_type: syndication_job_type::UPDATE.to_string(),
            priority: Some(3), // Lower priority for updates
            payload: serde_json::to_value(&payload).map_err(|e| e.to_string())?,
            scheduled_at: None,
            queue: Some(SYNDICATION_QUEUE.to_string()),
            max_attempts: Some(3),
            org_id: listing.organization_id,
        };

        let job = self
            .background_job_repo
            .create(job_data, created_by)
            .await
            .map_err(|e| {
                format!(
                    "Failed to create update job for portal {}: {}",
                    syndication.portal, e
                )
            })?;

        tracing::info!(
            listing_id = %listing.id,
            portal = %syndication.portal,
            job_id = %job.id,
            "Created syndication update job"
        );

        Ok(job.id)
    }

    /// Process a syndication job (would be called by a background worker).
    ///
    /// This is a placeholder for the actual portal API integration.
    /// In production, this would call the respective portal APIs.
    pub async fn process_syndication_job(
        &self,
        payload: &SyndicationJobPayload,
    ) -> Result<String, String> {
        // Get the listing (verify it exists)
        let _listing = self
            .listing_repo
            .find_by_id(payload.listing_id)
            .await
            .map_err(|e| format!("Failed to find listing: {}", e))?
            .ok_or_else(|| "Listing not found".to_string())?;

        // Get photos for the listing (for portal API calls)
        let photos = self
            .listing_repo
            .get_photos(payload.listing_id)
            .await
            .map_err(|e| format!("Failed to get listing photos: {}", e))?;

        match payload.operation.as_str() {
            syndication_job_type::PUBLISH => {
                // Simulate publishing to portal
                tracing::info!(
                    listing_id = %payload.listing_id,
                    portal = %payload.portal,
                    "Publishing listing to portal"
                );

                // Generate a mock external ID
                let external_id = format!("{}_{}", payload.portal, uuid::Uuid::new_v4());

                // Update syndication status
                self.listing_repo
                    .update_syndication_status(
                        payload.listing_id,
                        &payload.portal,
                        syndication_status::SYNCED,
                        Some(&external_id),
                        None,
                    )
                    .await
                    .map_err(|e| format!("Failed to update syndication status: {}", e))?;

                Ok(external_id)
            }
            syndication_job_type::STATUS_CHANGE => {
                let portal_status = payload
                    .new_status
                    .as_ref()
                    .and_then(|s| get_portal_status(s, &payload.portal));

                tracing::info!(
                    listing_id = %payload.listing_id,
                    portal = %payload.portal,
                    portal_status = ?portal_status,
                    "Propagating status change to portal"
                );

                // Update syndication timestamp
                self.listing_repo
                    .update_syndication_status(
                        payload.listing_id,
                        &payload.portal,
                        syndication_status::SYNCED,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| format!("Failed to update syndication status: {}", e))?;

                Ok("status_updated".to_string())
            }
            syndication_job_type::UPDATE => {
                tracing::info!(
                    listing_id = %payload.listing_id,
                    portal = %payload.portal,
                    photos_count = photos.len(),
                    "Updating listing on portal"
                );

                // Update syndication timestamp
                self.listing_repo
                    .update_syndication_status(
                        payload.listing_id,
                        &payload.portal,
                        syndication_status::SYNCED,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| format!("Failed to update syndication status: {}", e))?;

                Ok("updated".to_string())
            }
            syndication_job_type::REMOVE => {
                tracing::info!(
                    listing_id = %payload.listing_id,
                    portal = %payload.portal,
                    "Removing listing from portal"
                );

                // Mark syndication as removed
                self.listing_repo
                    .update_syndication_status(
                        payload.listing_id,
                        &payload.portal,
                        syndication_status::REMOVED,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| format!("Failed to update syndication status: {}", e))?;

                Ok("removed".to_string())
            }
            _ => Err(format!("Unknown operation: {}", payload.operation)),
        }
    }

    /// Drain up to `limit` due jobs from the syndication queue.
    ///
    /// Each job is claimed atomically (`claim_background_job` uses
    /// `FOR UPDATE SKIP LOCKED`, so replicas never double-process), run through
    /// [`Self::process_syndication_job`], then completed or failed (the DB
    /// function re-queues it until `max_attempts` is exhausted). Returns the
    /// number of jobs claimed.
    pub async fn drain_queue(&self, limit: usize) -> Result<usize, String> {
        let mut processed = 0;
        while processed < limit {
            let Some(job) = self
                .background_job_repo
                .claim_job(SYNDICATION_QUEUE, SYNDICATION_WORKER_ID, None)
                .await
                .map_err(|e| format!("Failed to claim syndication job: {}", e))?
            else {
                break;
            };
            processed += 1;

            let outcome = match serde_json::from_value::<SyndicationJobPayload>(job.payload.clone())
            {
                Ok(payload) => self.process_syndication_job(&payload).await,
                Err(e) => Err(format!("Invalid syndication job payload: {}", e)),
            };

            let finalize = match outcome {
                Ok(external_id) => {
                    self.background_job_repo
                        .complete_job(job.id, Some(json!({ "result": external_id })))
                        .await
                }
                Err(msg) => {
                    tracing::warn!(job_id = %job.id, error = %msg, "Syndication job failed");
                    self.background_job_repo.fail_job(job.id, &msg, None).await
                }
            };
            if let Err(e) = finalize {
                tracing::error!(job_id = %job.id, error = %e, "Failed to finalize syndication job");
            }
        }
        Ok(processed)
    }

    /// Get queue statistics for syndication jobs.
    pub async fn get_queue_stats(&self) -> Result<db::models::BackgroundJobQueueStats, String> {
        self.background_job_repo
            .get_queue_stats(SYNDICATION_QUEUE)
            .await
            .map_err(|e| format!("Failed to get queue stats: {}", e))
    }

    /// Build the listing data payload for portal APIs.
    pub fn build_listing_payload(&self, listing: &Listing) -> serde_json::Value {
        json!({
            "id": listing.id,
            "title": listing.title,
            "description": listing.description,
            "property_type": listing.property_type,
            "transaction_type": listing.transaction_type,
            "price": listing.price.to_string(),
            "currency": listing.currency,
            "is_negotiable": listing.is_negotiable,
            "size_sqm": listing.size_sqm.map(|d| d.to_string()),
            "rooms": listing.rooms,
            "bathrooms": listing.bathrooms,
            "floor": listing.floor,
            "total_floors": listing.total_floors,
            "address": {
                "street": listing.street,
                "city": listing.city,
                "postal_code": listing.postal_code,
                "country": listing.country,
            },
            "location": {
                "latitude": listing.latitude.map(|d| d.to_string()),
                "longitude": listing.longitude.map(|d| d.to_string()),
            },
            "features": listing.features,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_portal_status() {
        assert_eq!(
            get_portal_status(listing_status::ACTIVE, listing_portal::REALITY_PORTAL),
            Some("active")
        );
        assert_eq!(
            get_portal_status(listing_status::ACTIVE, listing_portal::SREALITY),
            Some("published")
        );
        assert_eq!(
            get_portal_status(listing_status::SOLD, listing_portal::BEZREALITKY),
            Some("sold")
        );
        assert_eq!(
            get_portal_status(listing_status::ARCHIVED, listing_portal::NEHNUTELNOSTI),
            Some("removed")
        );
        assert_eq!(get_portal_status(listing_status::DRAFT, "unknown"), None);
    }

    #[test]
    fn test_get_status_mapping() {
        let mapping = get_status_mapping(listing_status::PAUSED).unwrap();
        assert_eq!(mapping.internal_status, listing_status::PAUSED);
        assert_eq!(mapping.reality_portal, "paused");
        assert_eq!(mapping.sreality, "hidden");

        assert!(get_status_mapping(listing_status::DRAFT).is_none());
    }
}

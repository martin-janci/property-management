//! Listing model (Epic 15) - Real estate listing management.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Listing status enum.
pub mod listing_status {
    pub const DRAFT: &str = "draft";
    pub const ACTIVE: &str = "active";
    pub const PAUSED: &str = "paused";
    pub const SOLD: &str = "sold";
    pub const RENTED: &str = "rented";
    pub const ARCHIVED: &str = "archived";
}

/// Transaction type enum.
pub mod transaction_type {
    pub const SALE: &str = "sale";
    pub const RENT: &str = "rent";
}

/// Property type enum for listings (maps from unit_type).
pub mod property_type {
    pub const APARTMENT: &str = "apartment";
    pub const HOUSE: &str = "house";
    pub const COMMERCIAL: &str = "commercial";
    pub const LAND: &str = "land";
    pub const PARKING: &str = "parking";
    pub const STORAGE: &str = "storage";
    pub const OTHER: &str = "other";
}

/// Currency enum.
pub mod currency {
    pub const EUR: &str = "EUR";
    pub const CZK: &str = "CZK";
}

/// Listing entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Listing {
    pub id: Uuid,
    /// NULL for portal-direct listings (no PPT org). See migration 00186.
    pub organization_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub created_by: Uuid,
    /// Set for portal-user-created listings; NULL for org-owned listings.
    pub portal_owner_id: Option<Uuid>,

    // Listing status
    pub status: String,
    pub transaction_type: String,

    // Property details (can override unit data)
    pub title: String,
    pub description: Option<String>,
    pub property_type: String,
    // NULLable NUMERIC: decode natively as `Option<Decimal>`. A `try_from`
    // attribute here forces a non-null decode and panics with
    // `UnexpectedNullError` whenever the column is NULL (e.g. a portal listing
    // created without an area), so it must not be used on optional columns.
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,

    // Address (copied from unit/building or manually entered)
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,

    // Location coordinates (optional) — NULLable, decode natively (no try_from).
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,

    // Pricing
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub price: Decimal,
    pub currency: String,
    pub is_negotiable: bool,

    // Features (stored as JSON array)
    pub features: serde_json::Value,

    // Phase 4: explicit publish state for the global reality portal.
    // `is_published` is the toggle that opts a listing into the 4th RLS
    // context (PlatformHost / global-read across all orgs). It is orthogonal
    // to `status`: a listing can be `status='active'` on its agency subdomain
    // while `is_published=FALSE` (not on the global portal), and vice versa.
    // See migrations 00135/00136 and the ROADMAP Phase 4 section.
    #[serde(default)]
    pub is_published: bool,

    // Timestamps
    pub published_at: Option<DateTime<Utc>>,
    pub sold_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Listing {
    /// Check if listing is in draft status.
    pub fn is_draft(&self) -> bool {
        self.status == listing_status::DRAFT
    }

    /// Check if listing is active (visible to public).
    pub fn is_active(&self) -> bool {
        self.status == listing_status::ACTIVE
    }

    /// Check if listing can be edited.
    pub fn can_edit(&self) -> bool {
        matches!(
            self.status.as_str(),
            listing_status::DRAFT | listing_status::ACTIVE | listing_status::PAUSED
        )
    }

    /// Check if listing can be published.
    pub fn can_publish(&self) -> bool {
        self.status == listing_status::DRAFT || self.status == listing_status::PAUSED
    }

    /// Get features as a list of strings.
    pub fn feature_list(&self) -> Vec<String> {
        self.features
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Summary view of a listing (for list views).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ListingSummary {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub title: String,
    pub property_type: String,
    pub transaction_type: String,
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub price: Decimal,
    pub currency: String,
    // NULLable NUMERIC — decode natively (no try_from; see `Listing::size_sqm`).
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub city: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub photo_url: Option<String>,
}

/// Listing with full details including photos.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListingWithDetails {
    #[serde(flatten)]
    pub listing: Listing,
    pub photos: Vec<ListingPhoto>,
    pub syndications: Vec<ListingSyndication>,
}

/// Data for creating a new listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateListing {
    pub unit_id: Option<Uuid>,
    pub transaction_type: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(default = "default_property_type")]
    pub property_type: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    #[serde(default = "default_country")]
    pub country: String,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub price: Decimal,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub is_negotiable: bool,
    #[serde(default)]
    pub features: Vec<String>,
}

fn default_property_type() -> String {
    property_type::APARTMENT.to_string()
}

fn default_country() -> String {
    "SK".to_string()
}

fn default_currency() -> String {
    currency::EUR.to_string()
}

/// Data for creating a listing from unit data (pre-populated).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateListingFromUnit {
    pub unit_id: Uuid,
    pub transaction_type: String,
    pub price: Decimal,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub is_negotiable: bool,
    // Optional overrides (if not provided, use unit/building data)
    pub title: Option<String>,
    pub description: Option<String>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    #[serde(default)]
    pub features: Vec<String>,
}

/// Data for updating a listing.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateListing {
    pub title: Option<String>,
    pub description: Option<String>,
    pub property_type: Option<String>,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub price: Option<Decimal>,
    pub currency: Option<String>,
    pub is_negotiable: Option<bool>,
    pub features: Option<Vec<String>>,
}

/// Data for updating listing status.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateListingStatus {
    pub status: String,
}

/// Query parameters for listing search.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ListingListQuery {
    pub status: Option<String>,
    pub transaction_type: Option<String>,
    pub property_type: Option<String>,
    pub city: Option<String>,
    pub price_min: Option<Decimal>,
    pub price_max: Option<Decimal>,
    pub rooms_min: Option<i32>,
    pub rooms_max: Option<i32>,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

// ============================================
// Listing Photos
// ============================================

/// Listing photo entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ListingPhoto {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub medium_url: Option<String>,
    pub display_order: i32,
    pub alt_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data for adding a photo to a listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateListingPhoto {
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub medium_url: Option<String>,
    pub display_order: Option<i32>,
    pub alt_text: Option<String>,
}

/// Data for reordering photos.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReorderPhotos {
    pub photo_ids: Vec<Uuid>,
}

// ============================================
// Listing Syndication (Multi-Portal Publishing)
// ============================================

/// Syndication status enum.
pub mod syndication_status {
    pub const PENDING: &str = "pending";
    pub const SYNCED: &str = "synced";
    pub const FAILED: &str = "failed";
    pub const REMOVED: &str = "removed";
}

/// Portal enum.
pub mod portal {
    pub const REALITY_PORTAL: &str = "reality_portal";
    pub const SREALITY: &str = "sreality";
    pub const BEZREALITKY: &str = "bezrealitky";
    pub const NEHNUTELNOSTI: &str = "nehnutelnosti";
}

/// Listing syndication entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ListingSyndication {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub portal: String,
    pub external_id: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub synced_at: Option<DateTime<Utc>>,
    /// Cumulative portal view count (monotonic; advanced once per non-duplicate
    /// view webhook -- see `increment_syndication_stats` and #2360).
    #[serde(default)]
    pub total_views: i64,
    /// Cumulative portal inquiry count (monotonic).
    #[serde(default)]
    pub total_inquiries: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a syndication.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSyndication {
    pub portals: Vec<String>,
}

/// Syndication result for a single portal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyndicationResult {
    pub portal: String,
    pub success: bool,
    pub external_id: Option<String>,
    pub error: Option<String>,
}

/// Response for publish operation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishListingResponse {
    pub listing_id: Uuid,
    pub status: String,
    pub syndication_results: Vec<SyndicationResult>,
}

// ============================================
// Listing Statistics
// ============================================

/// Listing statistics for dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListingStatistics {
    pub total_listings: i64,
    pub active_listings: i64,
    pub draft_listings: i64,
    pub sold_listings: i64,
    pub rented_listings: i64,
    pub by_property_type: Vec<PropertyTypeCount>,
}

/// Count by property type.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PropertyTypeCount {
    pub property_type: String,
    pub count: i64,
}

// ============================================
// Epic 105: Portal Syndication (Extended)
// ============================================

/// Syndication job type constants.
pub mod syndication_job_type {
    pub const PUBLISH: &str = "syndication_publish";
    pub const UPDATE: &str = "syndication_update";
    pub const STATUS_CHANGE: &str = "syndication_status_change";
    pub const REMOVE: &str = "syndication_remove";
}

/// Syndication job payload for background processing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyndicationJobPayload {
    /// Listing ID to syndicate
    pub listing_id: Uuid,
    /// Target portal for syndication
    pub portal: String,
    /// Type of syndication operation
    pub operation: String,
    /// Previous status (for status change operations)
    pub previous_status: Option<String>,
    /// New status (for status change operations)
    pub new_status: Option<String>,
    /// Organization ID for context (None for portal-direct listings).
    pub organization_id: Option<Uuid>,
}

/// Syndication status dashboard response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyndicationStatusDashboard {
    /// Listing ID
    pub listing_id: Uuid,
    /// Listing title
    pub listing_title: String,
    /// Current listing status
    pub listing_status: String,
    /// Syndication status per portal
    pub portal_statuses: Vec<PortalSyndicationStatus>,
    /// Overall syndication health
    pub overall_status: SyndicationHealthStatus,
    /// Total views across all portals
    pub total_views: i64,
    /// Total inquiries across all portals
    pub total_inquiries: i64,
}

/// Syndication status for a specific portal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalSyndicationStatus {
    /// Portal name
    pub portal: String,
    /// Current syndication status
    pub status: String,
    /// External listing ID on the portal
    pub external_id: Option<String>,
    /// Last successful sync timestamp
    pub synced_at: Option<DateTime<Utc>>,
    /// Last error message if any
    pub last_error: Option<String>,
    /// Number of views from this portal
    pub views: i64,
    /// Number of inquiries from this portal
    pub inquiries: i64,
    /// Last activity timestamp
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// Overall syndication health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyndicationHealthStatus {
    /// All portals synced successfully
    Healthy,
    /// Some portals have issues
    Degraded,
    /// All portals failing
    Unhealthy,
    /// No syndications configured
    NotConfigured,
}

/// Portal webhook event types.
pub mod webhook_event_type {
    pub const VIEW: &str = "view";
    pub const INQUIRY: &str = "inquiry";
    pub const FAVORITE: &str = "favorite";
    pub const PRICE_ALERT: &str = "price_alert";
    pub const STATUS_UPDATE: &str = "status_update";
}

/// Portal webhook event.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalWebhookEvent {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub syndication_id: Uuid,
    pub portal: String,
    pub event_type: String,
    pub external_id: Option<String>,
    pub payload: serde_json::Value,
    pub processed: bool,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a portal webhook event.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePortalWebhookEvent {
    /// Portal sending the webhook
    pub portal: String,
    /// Event type
    pub event_type: String,
    /// External listing ID on the portal
    pub external_id: String,
    /// Event payload from the portal
    pub payload: serde_json::Value,
}

/// Portal webhook incoming request for views/inquiries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalViewWebhook {
    /// External listing ID on the portal
    pub external_id: String,
    /// Number of views to add
    #[serde(default)]
    pub views_count: i64,
    /// Timestamp of the view event
    pub timestamp: Option<DateTime<Utc>>,
}

/// Portal webhook incoming request for inquiries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalInquiryWebhook {
    /// External listing ID on the portal
    pub external_id: String,
    /// Inquiry sender name
    pub sender_name: Option<String>,
    /// Inquiry sender email
    pub sender_email: Option<String>,
    /// Inquiry sender phone
    pub sender_phone: Option<String>,
    /// Inquiry message
    pub message: Option<String>,
    /// Timestamp of the inquiry
    pub timestamp: Option<DateTime<Utc>>,
}

/// Aggregated syndication statistics for an organization.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrganizationSyndicationStats {
    /// Total active syndications
    pub total_active: i64,
    /// Total pending syndications
    pub total_pending: i64,
    /// Total failed syndications
    pub total_failed: i64,
    /// Total views across all portals
    pub total_views: i64,
    /// Total inquiries across all portals
    pub total_inquiries: i64,
    /// Stats breakdown by portal
    pub by_portal: Vec<PortalStats>,
}

/// Statistics for a specific portal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalStats {
    /// Portal name
    pub portal: String,
    /// Number of active listings on this portal
    pub active_count: i64,
    /// Number of pending listings
    pub pending_count: i64,
    /// Number of failed listings
    pub failed_count: i64,
    /// Total views
    pub views: i64,
    /// Total inquiries
    pub inquiries: i64,
}

/// Query parameters for syndication dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct SyndicationDashboardQuery {
    /// Filter by portal
    pub portal: Option<String>,
    /// Filter by syndication status
    pub status: Option<String>,
    /// Filter by date range start
    pub from_date: Option<DateTime<Utc>>,
    /// Filter by date range end
    pub to_date: Option<DateTime<Utc>>,
    /// Page number
    pub page: Option<i32>,
    /// Items per page
    pub limit: Option<i32>,
}

/// Response for syndication dashboard listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyndicationDashboardResponse {
    /// List of listing syndication statuses
    pub listings: Vec<SyndicationStatusDashboard>,
    /// Total count
    pub total: i64,
    /// Current page
    pub page: i32,
    /// Items per page
    pub limit: i32,
    /// Organization-wide statistics
    pub organization_stats: OrganizationSyndicationStats,
}

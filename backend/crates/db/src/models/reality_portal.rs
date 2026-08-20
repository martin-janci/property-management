//! Reality Portal Professional models (Epics 31-34).
//!
//! Models for agencies, realtors, inquiries, and property import.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================
// Portal Favorites Enhanced (Story 31.1, 31.4)
// ============================================

/// Portal favorite with price tracking.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalFavorite {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub notes: Option<String>,
    pub original_price: Option<Decimal>,
    pub price_alert_enabled: bool,
    pub last_price_alert_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Favorite with listing details and price change info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalFavoriteWithListing {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub title: String,
    pub current_price: Decimal,
    pub original_price: Option<Decimal>,
    pub currency: String,
    pub city: String,
    pub property_type: String,
    pub transaction_type: String,
    pub photo_url: Option<String>,
    pub status: String,
    pub price_changed: bool,
    pub price_change_percentage: Option<Decimal>,
    pub price_alert_enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Update favorite request (price alerts).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePortalFavorite {
    pub notes: Option<String>,
    pub price_alert_enabled: Option<bool>,
}

// ============================================
// Price History (Story 31.4)
// ============================================

/// Listing price history entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ListingPriceHistory {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub old_price: Decimal,
    pub new_price: Decimal,
    pub currency: String,
    pub change_percentage: Option<Decimal>,
    pub changed_at: DateTime<Utc>,
}

/// Price change alert.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PriceChangeAlert {
    pub listing_id: Uuid,
    pub title: String,
    pub old_price: Decimal,
    pub new_price: Decimal,
    pub currency: String,
    pub change_percentage: Decimal,
    pub changed_at: DateTime<Utc>,
}

/// Favorite alert queued for in-app delivery.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FavoriteAlert {
    pub id: Uuid,
    pub favorite_id: Uuid,
    pub listing_id: Uuid,
    pub title: String,
    pub alert_type: String,
    pub old_price: Option<Decimal>,
    pub new_price: Option<Decimal>,
    pub currency: Option<String>,
    pub change_percentage: Option<Decimal>,
    pub previous_status: Option<String>,
    pub new_status: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

// ============================================
// Portal Saved Searches Enhanced (Story 31.2, 31.3)
// ============================================

/// Portal saved search with alert configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalSavedSearch {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub criteria: serde_json::Value,
    pub alerts_enabled: bool,
    pub alert_frequency: String,
    pub last_matched_at: Option<DateTime<Utc>>,
    /// Running total of listings matched by this saved search across every
    /// alert run. Widened to `bigint` (i64) in migration 00232 so a long-lived,
    /// high-traffic search can never wedge itself on integer overflow — see
    /// #2814 and the `saved_search_watermark_advance_tests` suite.
    pub match_count: i64,
    pub last_alert_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A queued saved-search alert as surfaced to the owning portal user — the
/// in-app delivery view of a `search_alert_queue` row joined to its saved
/// search's name (Story 16.3, issue #983). The background matching engine
/// (`SavedSearchAlertWorker`) enqueues these; this is how the user retrieves
/// them.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SavedSearchAlert {
    pub id: Uuid,
    pub saved_search_id: Uuid,
    /// Name of the saved search that produced the alert (joined).
    pub saved_search_name: String,
    /// Listings that newly matched the saved search.
    pub matching_listing_ids: Vec<Uuid>,
    /// `new_listing` (today); `price_change` is reserved for the favorites path.
    pub alert_type: String,
    /// `pending` (undelivered) | `sent` (read) | `failed`.
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

/// An undelivered saved-search alert as seen by the email/push transport drainer
/// (BIT-139, Epic 16). This is the *out-of-band* delivery view of a
/// `search_alert_queue` row: it carries the recipient's contact details
/// (resolved from `users`) and the saved-search name so the drainer can compose
/// a notification without a second round-trip.
///
/// Delivery here is tracked by `search_alert_queue.notified_at` /
/// `notify_attempts` (migration 00195), independently of the `status` column,
/// which belongs to the in-app pull/read channel ([`SavedSearchAlert`]).
#[derive(Debug, Clone, FromRow)]
pub struct UndeliveredSearchAlert {
    pub id: Uuid,
    pub saved_search_id: Uuid,
    pub user_id: Uuid,
    /// Name of the saved search that produced the alert (joined).
    pub saved_search_name: String,
    /// Listings that newly matched the saved search.
    pub matching_listing_ids: Vec<Uuid>,
    /// `new_listing` (today); `price_change` is reserved for the favorites path.
    pub alert_type: String,
    /// Recipient email, resolved from `users` (not RLS-gated).
    pub recipient_email: String,
    /// Recipient display name, for the email greeting.
    pub recipient_name: String,
    /// Recipient locale (`sk` | `cs` | `de` | `en`) for future localized copy.
    pub recipient_locale: Option<String>,
}

/// Create portal saved search.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePortalSavedSearch {
    pub name: String,
    pub criteria: serde_json::Value,
    #[serde(default = "default_true")]
    pub alerts_enabled: bool,
    #[serde(default = "default_daily")]
    pub alert_frequency: String,
}

/// Update portal saved search.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdatePortalSavedSearch {
    pub name: Option<String>,
    pub criteria: Option<serde_json::Value>,
    pub alerts_enabled: Option<bool>,
    pub alert_frequency: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_daily() -> String {
    "daily".to_string()
}

// ============================================
// Reality Agencies (Story 32.1, 32.4)
// ============================================

/// Reality agency entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RealityAgency {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub email: String,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub logo_watermark_position: Option<String>,
    pub description: Option<String>,
    pub tagline: Option<String>,
    pub status: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub plan: String,
    pub max_listings: i32,
    pub max_realtors: i32,
    pub total_listings: i32,
    pub active_listings: i32,
    pub total_views: i32,
    pub total_inquiries: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create agency request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRealityAgency {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub description: Option<String>,
    pub tagline: Option<String>,
}

/// Update agency request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateRealityAgency {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub description: Option<String>,
    pub tagline: Option<String>,
}

/// Update agency branding request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateAgencyBranding {
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub logo_watermark_position: Option<String>,
}

/// Agency summary for listings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgencySummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    pub primary_color: Option<String>,
}

// ============================================
// Agency Members (Story 32.2)
// ============================================

/// Agency member entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RealityAgencyMember {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

/// Agency member with user info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgencyMemberWithUser {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    pub profile_image_url: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
}

/// Agency invitation.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RealityAgencyInvitation {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub email: String,
    pub role: String,
    pub invited_by: Uuid,
    pub token: String,
    pub message: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create invitation request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAgencyInvitation {
    pub email: String,
    pub role: Option<String>,
    pub message: Option<String>,
}

// ============================================
// Realtor Profiles (Story 33.1)
// ============================================

/// Realtor profile entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RealtorProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub photo_url: Option<String>,
    pub bio: Option<String>,
    pub tagline: Option<String>,
    pub specializations: Option<Vec<String>>,
    pub experience_years: Option<i32>,
    pub languages: Option<Vec<String>>,
    pub license_number: Option<String>,
    pub license_verified_at: Option<DateTime<Utc>>,
    pub certifications: Option<Vec<String>>,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email_public: Option<String>,
    pub linkedin_url: Option<String>,
    pub facebook_url: Option<String>,
    pub instagram_url: Option<String>,
    pub show_phone: bool,
    pub show_email: bool,
    pub accept_inquiries: bool,
    pub total_listings: i32,
    pub active_listings: i32,
    pub total_views: i32,
    pub total_inquiries: i32,
    pub avg_response_time_hours: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create realtor profile request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRealtorProfile {
    pub bio: Option<String>,
    pub tagline: Option<String>,
    pub specializations: Option<Vec<String>>,
    pub experience_years: Option<i32>,
    pub languages: Option<Vec<String>>,
    pub license_number: Option<String>,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email_public: Option<String>,
}

/// Update realtor profile request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateRealtorProfile {
    pub photo_url: Option<String>,
    pub bio: Option<String>,
    pub tagline: Option<String>,
    pub specializations: Option<Vec<String>>,
    pub experience_years: Option<i32>,
    pub languages: Option<Vec<String>>,
    pub license_number: Option<String>,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email_public: Option<String>,
    pub linkedin_url: Option<String>,
    pub facebook_url: Option<String>,
    pub instagram_url: Option<String>,
    pub show_phone: Option<bool>,
    pub show_email: Option<bool>,
    pub accept_inquiries: Option<bool>,
}

/// Public realtor profile.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicRealtorProfile {
    pub id: Uuid,
    pub name: String,
    pub photo_url: Option<String>,
    pub bio: Option<String>,
    pub tagline: Option<String>,
    pub specializations: Vec<String>,
    pub experience_years: Option<i32>,
    pub languages: Vec<String>,
    pub license_verified: bool,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
    pub linkedin_url: Option<String>,
    pub agency: Option<AgencySummary>,
    pub active_listings: i32,
}

// ============================================
// Realtor Listings (Story 33.2)
// ============================================

/// Realtor listing assignment.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RealtorListing {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub realtor_id: Uuid,
    pub agency_id: Option<Uuid>,
    pub visibility: String,
    pub show_realtor_info: bool,
    pub show_agency_branding: bool,
    pub is_featured: bool,
    pub featured_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Assign listing request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignRealtorListing {
    pub listing_id: Uuid,
    pub visibility: Option<String>,
    pub show_realtor_info: Option<bool>,
    pub show_agency_branding: Option<bool>,
}

// ============================================
// Listing Inquiries (Story 33.3)
// ============================================

/// Listing inquiry entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ListingInquiry {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub realtor_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub message: String,
    pub inquiry_type: String,
    pub preferred_contact: String,
    pub preferred_time: Option<String>,
    pub status: String,
    pub read_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Create inquiry request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateListingInquiry {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub message: String,
    pub inquiry_type: Option<String>,
    pub preferred_contact: Option<String>,
    pub preferred_time: Option<String>,
}

/// Inquiry message.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct InquiryMessage {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub sender_type: String,
    pub sender_id: Uuid,
    pub message: String,
    pub attachments: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Send inquiry message request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SendInquiryMessage {
    pub message: String,
}

/// Inquiry with listing info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InquiryWithListing {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub listing_title: String,
    pub listing_photo_url: Option<String>,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub message: String,
    pub inquiry_type: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub message_count: i32,
}

// ============================================
// Viewing Schedules (Story 33.3)
// ============================================

/// Viewing schedule entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ViewingSchedule {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub listing_id: Uuid,
    pub realtor_id: Uuid,
    pub attendee_name: String,
    pub attendee_email: String,
    pub attendee_phone: Option<String>,
    pub scheduled_at: DateTime<Utc>,
    pub duration_minutes: i32,
    pub status: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub internal_notes: Option<String>,
    pub meeting_notes: Option<String>,
    pub reminder_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Schedule viewing request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleViewing {
    pub scheduled_at: DateTime<Utc>,
    pub duration_minutes: Option<i32>,
    pub internal_notes: Option<String>,
}

/// Update viewing request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateViewing {
    pub scheduled_at: Option<DateTime<Utc>>,
    pub duration_minutes: Option<i32>,
    pub status: Option<String>,
    pub cancellation_reason: Option<String>,
    pub meeting_notes: Option<String>,
}

// ============================================
// Listing Analytics (Story 33.4)
// ============================================

/// Listing analytics entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ListingAnalytics {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub date: NaiveDate,
    pub views: i32,
    pub unique_views: i32,
    pub favorites_added: i32,
    pub favorites_removed: i32,
    pub inquiries: i32,
    pub phone_clicks: i32,
    pub share_clicks: i32,
    pub source_website: i32,
    pub source_mobile: i32,
    pub source_search: i32,
    pub source_direct: i32,
    pub created_at: DateTime<Utc>,
}

/// Listing analytics summary.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListingAnalyticsSummary {
    pub listing_id: Uuid,
    pub total_views: i64,
    pub total_inquiries: i64,
    pub total_favorites: i64,
    pub days_on_market: i32,
    pub daily_analytics: Vec<ListingAnalytics>,
}

// ============================================
// CRM Connections (Story 34.2)
// ============================================

/// CRM connection entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CrmConnection {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub crm_type: String,
    pub name: String,
    pub api_endpoint: Option<String>,
    pub field_mapping: serde_json::Value,
    pub sync_enabled: bool,
    pub sync_frequency: String,
    pub sync_direction: String,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create CRM connection request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCrmConnection {
    pub crm_type: String,
    pub name: String,
    pub api_endpoint: Option<String>,
    pub api_key: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub field_mapping: Option<serde_json::Value>,
    pub sync_frequency: Option<String>,
    pub sync_direction: Option<String>,
}

/// Update CRM connection request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateCrmConnection {
    pub name: Option<String>,
    pub api_endpoint: Option<String>,
    pub field_mapping: Option<serde_json::Value>,
    pub sync_enabled: Option<bool>,
    pub sync_frequency: Option<String>,
    pub sync_direction: Option<String>,
}

// ============================================
// Import Jobs (Story 34.1, 34.3, 34.4)
// ============================================

/// Import job entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalImportJob {
    pub id: Uuid,
    pub agency_id: Option<Uuid>,
    pub user_id: Uuid,
    pub source_type: String,
    pub source_url: Option<String>,
    pub source_filename: Option<String>,
    pub crm_connection_id: Option<Uuid>,
    pub status: String,
    pub total_records: i32,
    pub processed_records: i32,
    pub success_count: i32,
    pub skip_count: i32,
    pub failure_count: i32,
    pub created_listings: Option<Vec<Uuid>>,
    pub updated_listings: Option<Vec<Uuid>>,
    pub error_log: Option<serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create import job request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateImportJob {
    pub agency_id: Option<Uuid>,
    pub source_type: String,
    pub source_url: Option<String>,
    pub source_filename: Option<String>,
}

/// Update import job request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateImportJob {
    pub source_url: Option<String>,
    pub source_filename: Option<String>,
}

/// Import job with stats for listing.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalImportJobWithStats {
    pub id: Uuid,
    pub source_type: String,
    pub source_url: Option<String>,
    pub source_filename: Option<String>,
    pub status: String,
    pub total_records: i32,
    pub processed_records: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Import job progress.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImportJobProgress {
    pub id: Uuid,
    pub status: String,
    pub total_records: i32,
    pub processed_records: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub progress_percentage: i32,
}

// ============================================
// Feed Subscriptions (Story 34.4)
// ============================================

/// Feed subscription entity (alias for routes compatibility).
pub type RealityFeedSubscription = FeedSubscription;

/// Feed subscription entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeedSubscription {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    pub feed_url: String,
    pub feed_type: String,
    pub sync_interval: String,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub consecutive_failures: i32,
    pub total_imported: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create feed subscription request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFeedSubscription {
    pub name: String,
    pub feed_url: String,
    pub feed_type: Option<String>,
    pub sync_interval: Option<String>,
}

/// Update feed subscription request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateFeedSubscription {
    pub name: Option<String>,
    pub feed_url: Option<String>,
    pub feed_type: Option<String>,
    pub sync_interval: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================
// Search Alert Queue
// ============================================

/// Search alert queue entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SearchAlertQueueEntry {
    pub id: Uuid,
    pub saved_search_id: Uuid,
    pub user_id: Uuid,
    pub matching_listing_ids: Vec<Uuid>,
    pub alert_type: String,
    pub status: String,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

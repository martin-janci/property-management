//! Rental models (Epic 18: Short-Term Rental Integration).
//!
//! Models for Airbnb/Booking.com integration, guest registration, and authority reports.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================
// Platform Type
// ============================================

/// Rental platform types.
pub mod rental_platform {
    pub const AIRBNB: &str = "airbnb";
    pub const BOOKING: &str = "booking";
    pub const VRBO: &str = "vrbo";
    pub const DIRECT: &str = "direct";
}

// ============================================
// Booking Status
// ============================================

/// Rental booking status.
pub mod booking_status {
    pub const PENDING: &str = "pending";
    pub const CONFIRMED: &str = "confirmed";
    pub const CHECKED_IN: &str = "checked_in";
    pub const CHECKED_OUT: &str = "checked_out";
    pub const CANCELLED: &str = "cancelled";
    pub const NO_SHOW: &str = "no_show";
}

// ============================================
// Guest Registration Status
// ============================================

/// Guest registration status.
pub mod guest_status {
    pub const PENDING: &str = "pending";
    pub const REGISTERED: &str = "registered";
    pub const REPORTED: &str = "reported";
    pub const EXPIRED: &str = "expired";
}

// ============================================
// Calendar Block Reasons
// ============================================

/// Calendar block reasons.
pub mod block_reason {
    pub const BOOKING: &str = "booking";
    pub const MAINTENANCE: &str = "maintenance";
    pub const OWNER_USE: &str = "owner_use";
    pub const BLOCKED: &str = "blocked";
}

// ============================================
// Report Types
// ============================================

/// Guest report types.
pub mod report_type {
    pub const MONTHLY: &str = "monthly";
    pub const QUARTERLY: &str = "quarterly";
    pub const ANNUAL: &str = "annual";
}

/// Report status.
pub mod report_status {
    pub const DRAFT: &str = "draft";
    pub const GENERATED: &str = "generated";
    pub const SUBMITTED: &str = "submitted";
    pub const CONFIRMED: &str = "confirmed";
}

/// Authority codes.
pub mod authority_code {
    pub const SK_UHUL: &str = "SK_UHUL"; // Slovakia tourism
    pub const CZ_CIZPOL: &str = "CZ_CIZPOL"; // Czech foreign police
    pub const AT_ZMR: &str = "AT_ZMR"; // Austria registration
    pub const DE_MELDEWESEN: &str = "DE_MELDEWESEN"; // Germany registration
}

// ============================================
// Platform Connection (Story 18.1)
// ============================================

/// Platform connection entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RentalPlatformConnection {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub platform: String, // airbnb, booking, vrbo, direct

    // OAuth credentials (encrypted at application level)
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,

    // Gap 83-1: Airbnb-specific encrypted token columns (canonical after migration 00175).
    // Non-Airbnb connections leave these NULL.
    pub encrypted_token: Option<String>,
    pub encrypted_refresh_token: Option<String>,

    // External identifiers
    pub external_property_id: Option<String>,
    pub external_listing_url: Option<String>,

    // Connection status
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_error: Option<String>,

    // Calendar sync settings
    pub sync_calendar: bool,
    pub sync_interval_minutes: i32,
    pub block_other_platforms: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RentalPlatformConnection {
    pub fn is_synced(&self) -> bool {
        self.last_sync_at.is_some() && self.sync_error.is_none()
    }

    pub fn needs_token_refresh(&self) -> bool {
        self.token_expires_at
            .map(|exp| exp <= Utc::now())
            .unwrap_or(true)
    }

    /// Return the encrypted access token, preferring the canonical
    /// `encrypted_token` column (gap-83-1) over the legacy `access_token`.
    pub fn canonical_encrypted_token(&self) -> Option<&str> {
        self.encrypted_token
            .as_deref()
            .or(self.access_token.as_deref())
    }

    /// Return the encrypted refresh token, preferring the canonical
    /// `encrypted_refresh_token` column (gap-83-1) over the legacy `refresh_token`.
    pub fn canonical_encrypted_refresh_token(&self) -> Option<&str> {
        self.encrypted_refresh_token
            .as_deref()
            .or(self.refresh_token.as_deref())
    }
}

/// Create platform connection request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePlatformConnection {
    pub unit_id: Uuid,
    pub platform: String,
    pub external_property_id: Option<String>,
    #[serde(default = "default_sync_calendar")]
    pub sync_calendar: bool,
    #[serde(default = "default_sync_interval")]
    pub sync_interval_minutes: i32,
    #[serde(default = "default_block_other")]
    pub block_other_platforms: bool,
}

fn default_sync_calendar() -> bool {
    true
}

fn default_sync_interval() -> i32 {
    15
}

fn default_block_other() -> bool {
    true
}

/// Update platform connection request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdatePlatformConnection {
    pub external_property_id: Option<String>,
    pub external_listing_url: Option<String>,
    pub is_active: Option<bool>,
    pub sync_calendar: Option<bool>,
    pub sync_interval_minutes: Option<i32>,
    pub block_other_platforms: Option<bool>,
}

/// OAuth callback data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

/// Connection status response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionStatus {
    pub id: Uuid,
    pub platform: String,
    pub is_connected: bool,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_error: Option<String>,
    pub external_listing_url: Option<String>,
}

/// Platform connection detail (token-free).
///
/// SECURITY (#887 / #804): the GET `/connections/{id}` endpoint must never
/// serialize `access_token` / `refresh_token`. This DTO mirrors
/// [`RentalPlatformConnection`] minus the OAuth secret fields and exposes a
/// boolean `is_connected` derived from token presence instead. Build it via
/// `PlatformConnectionDetail::from(connection)`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformConnectionDetail {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub platform: String,

    /// True when an access token is stored (without exposing the token itself).
    pub is_connected: bool,
    pub token_expires_at: Option<DateTime<Utc>>,

    pub external_property_id: Option<String>,
    pub external_listing_url: Option<String>,

    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_error: Option<String>,

    pub sync_calendar: bool,
    pub sync_interval_minutes: i32,
    pub block_other_platforms: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RentalPlatformConnection> for PlatformConnectionDetail {
    fn from(c: RentalPlatformConnection) -> Self {
        Self {
            id: c.id,
            organization_id: c.organization_id,
            unit_id: c.unit_id,
            platform: c.platform,
            is_connected: c.access_token.is_some(),
            token_expires_at: c.token_expires_at,
            external_property_id: c.external_property_id,
            external_listing_url: c.external_listing_url,
            is_active: c.is_active,
            last_sync_at: c.last_sync_at,
            sync_error: c.sync_error,
            sync_calendar: c.sync_calendar,
            sync_interval_minutes: c.sync_interval_minutes,
            block_other_platforms: c.block_other_platforms,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// Platform connection summary.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PlatformConnectionSummary {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_name: String,
    pub platform: String,
    pub is_active: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_error: Option<String>,
}

// ============================================
// Rental Booking (Story 18.2)
// ============================================

/// Rental booking entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RentalBooking {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub connection_id: Option<Uuid>,

    // Platform info
    pub platform: String, // airbnb, booking, vrbo, direct
    pub external_booking_id: Option<String>,
    pub external_booking_url: Option<String>,

    // Guest info
    pub guest_name: String,
    pub guest_email: Option<String>,
    pub guest_phone: Option<String>,
    pub guest_count: i32,

    // Booking dates
    pub check_in: NaiveDate,
    pub check_out: NaiveDate,
    pub check_in_time: Option<NaiveTime>,
    pub check_out_time: Option<NaiveTime>,

    // Financial — these DECIMAL columns are NULLABLE (migration 00051), so they
    // must decode as native nullable NUMERIC. `#[sqlx(try_from = "Decimal")]`
    // forces a NON-NULL `Decimal` decode first, which fails with "unexpected
    // null" on any booking row whose amount/fee is NULL (issue #1422). Plain
    // `Option<Decimal>` decodes nullable NUMERIC natively.
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub platform_fee: Option<Decimal>,
    pub cleaning_fee: Option<Decimal>,

    // Status
    pub status: String, // pending, confirmed, checked_in, checked_out, cancelled, no_show
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,

    // Notes
    pub guest_notes: Option<String>,
    pub internal_notes: Option<String>,

    // Sync metadata
    pub synced_at: Option<DateTime<Utc>>,
    pub raw_data: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RentalBooking {
    pub fn nights(&self) -> i64 {
        (self.check_out - self.check_in).num_days()
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status.as_str(),
            booking_status::PENDING | booking_status::CONFIRMED | booking_status::CHECKED_IN
        )
    }

    pub fn is_upcoming(&self) -> bool {
        self.check_in > chrono::Local::now().date_naive() && self.is_active()
    }
}

/// Create booking request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBooking {
    pub unit_id: Uuid,
    #[serde(default = "default_platform")]
    pub platform: String,
    pub external_booking_id: Option<String>,
    pub guest_name: String,
    pub guest_email: Option<String>,
    pub guest_phone: Option<String>,
    #[serde(default = "default_guest_count")]
    pub guest_count: i32,
    pub check_in: NaiveDate,
    pub check_out: NaiveDate,
    pub check_in_time: Option<NaiveTime>,
    pub check_out_time: Option<NaiveTime>,
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub platform_fee: Option<Decimal>,
    pub cleaning_fee: Option<Decimal>,
    pub guest_notes: Option<String>,
    pub internal_notes: Option<String>,
}

fn default_platform() -> String {
    rental_platform::DIRECT.to_string()
}

fn default_guest_count() -> i32 {
    1
}

/// Update booking request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateBooking {
    pub guest_name: Option<String>,
    pub guest_email: Option<String>,
    pub guest_phone: Option<String>,
    pub guest_count: Option<i32>,
    pub check_in: Option<NaiveDate>,
    pub check_out: Option<NaiveDate>,
    pub check_in_time: Option<NaiveTime>,
    pub check_out_time: Option<NaiveTime>,
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub guest_notes: Option<String>,
    pub internal_notes: Option<String>,
}

/// Update booking status request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBookingStatus {
    pub status: String,
    pub cancellation_reason: Option<String>,
}

/// Booking summary for lists.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookingSummary {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_name: String,
    pub building_name: String,
    pub platform: String,
    /// External platform reservation ID (e.g. Booking.com reservation ID).
    pub external_booking_id: Option<String>,
    pub guest_name: String,
    pub guest_count: i32,
    pub check_in: NaiveDate,
    pub check_out: NaiveDate,
    pub nights: i64,
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub status: String,
    pub guest_registration_status: Option<String>,
}

/// Booking list query parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct BookingListQuery {
    pub unit_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub platform: Option<String>,
    pub status: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub guest_name: Option<String>,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

/// Bookings list response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookingsResponse {
    pub bookings: Vec<BookingSummary>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}

// ============================================
// Guest Registration (Story 18.3)
// ============================================

/// Rental guest entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RentalGuest {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub booking_id: Uuid,

    // Personal info
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub nationality: Option<String>, // ISO 3166-1 alpha-3

    // Identification
    pub id_type: Option<String>, // passport, national_id, driving_license
    pub id_number: Option<String>,
    pub id_issuing_country: Option<String>,
    pub id_expiry_date: Option<NaiveDate>,
    pub id_document_url: Option<String>,

    // Contact
    pub email: Option<String>,
    pub phone: Option<String>,

    // Address
    pub address_street: Option<String>,
    pub address_city: Option<String>,
    pub address_postal_code: Option<String>,
    pub address_country: Option<String>,

    // Registration status
    pub status: String, // pending, registered, reported, expired
    pub registered_at: Option<DateTime<Utc>>,
    pub reported_at: Option<DateTime<Utc>>,
    pub report_reference: Option<String>,

    // Primary guest flag
    pub is_primary: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RentalGuest {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_registered(&self) -> bool {
        self.status == guest_status::REGISTERED || self.status == guest_status::REPORTED
    }
}

/// Create guest request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateGuest {
    pub booking_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub nationality: Option<String>,
    pub id_type: Option<String>,
    pub id_number: Option<String>,
    pub id_issuing_country: Option<String>,
    pub id_expiry_date: Option<NaiveDate>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_street: Option<String>,
    pub address_city: Option<String>,
    pub address_postal_code: Option<String>,
    pub address_country: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
}

/// Update guest request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateGuest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub nationality: Option<String>,
    pub id_type: Option<String>,
    pub id_number: Option<String>,
    pub id_issuing_country: Option<String>,
    pub id_expiry_date: Option<NaiveDate>,
    pub id_document_url: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address_street: Option<String>,
    pub address_city: Option<String>,
    pub address_postal_code: Option<String>,
    pub address_country: Option<String>,
}

/// Register guest (mark as registered).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterGuest {
    pub guest_id: Uuid,
}

/// Guest summary for lists.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GuestSummary {
    pub id: Uuid,
    pub booking_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub nationality: Option<String>,
    pub id_type: Option<String>,
    pub status: String,
    pub is_primary: bool,
}

/// Booking with guests detail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookingWithGuests {
    pub booking: RentalBooking,
    pub guests: Vec<RentalGuest>,
    pub registration_complete: bool,
}

/// Guest check-in reminder notification.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckInReminder {
    pub booking_id: Uuid,
    pub unit_name: String,
    pub guest_name: String,
    pub check_in: NaiveDate,
    pub pending_registrations: i32,
}

// ============================================
// Authority Reports (Story 18.4)
// ============================================

/// Guest report entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RentalGuestReport {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,

    // Report period
    pub report_type: String, // monthly, quarterly, annual
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,

    // Authority info
    pub authority_code: String,
    pub authority_name: String,

    // Report content
    pub total_guests: i32,
    pub guests_by_nationality: Option<serde_json::Value>,

    // Generated files
    pub report_file_url: Option<String>,
    pub report_format: String, // pdf, csv, xml

    // Submission status
    pub status: String, // draft, generated, submitted, confirmed
    pub submitted_at: Option<DateTime<Utc>>,
    pub submission_reference: Option<String>,
    pub submission_response: Option<String>,

    // Metadata
    pub generated_by: Option<Uuid>,
    pub submitted_by: Option<Uuid>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Generate report request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateReport {
    pub building_id: Uuid,
    pub report_type: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub authority_code: String,
    #[serde(default = "default_report_format")]
    pub report_format: String,
}

fn default_report_format() -> String {
    "pdf".to_string()
}

/// Submit report request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitReport {
    pub report_id: Uuid,
}

/// Report summary for lists.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportSummary {
    pub id: Uuid,
    pub building_id: Uuid,
    pub building_name: String,
    pub report_type: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub authority_code: String,
    pub authority_name: String,
    pub total_guests: i32,
    pub status: String,
    pub report_file_url: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
}

/// Guest nationality statistics for reports.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NationalityStats {
    pub nationality: String,
    pub country_name: String,
    pub count: i32,
    pub percentage: f64,
}

/// Report preview before generation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportPreview {
    pub building_id: Uuid,
    pub building_name: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_guests: i32,
    pub by_nationality: Vec<NationalityStats>,
    pub bookings_count: i32,
}

// ============================================
// Calendar Blocks
// ============================================

/// Calendar block entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CalendarBlock {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub block_start: NaiveDate,
    pub block_end: NaiveDate,
    pub reason: String, // booking, maintenance, owner_use, blocked
    pub booking_id: Option<Uuid>,
    pub source_platform: Option<String>,
    pub synced_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Create calendar block request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCalendarBlock {
    pub unit_id: Uuid,
    pub block_start: NaiveDate,
    pub block_end: NaiveDate,
    pub reason: String,
    pub notes: Option<String>,
}

/// Calendar availability for a unit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnitAvailability {
    pub unit_id: Uuid,
    pub unit_name: String,
    pub blocks: Vec<CalendarBlock>,
    pub bookings: Vec<BookingSummary>,
}

/// Calendar event for unified view.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CalendarEvent {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub event_type: String, // booking, block
    pub title: String,
    pub platform: Option<String>,
    pub booking_status: Option<String>,
    pub color: String, // For UI display
}

// ============================================
// iCal Feeds
// ============================================

/// iCal feed entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ICalFeed {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub feed_name: String,
    pub feed_token: String,
    pub feed_url: Option<String>,
    pub import_url: Option<String>,
    pub import_platform: Option<String>,
    pub last_import_at: Option<DateTime<Utc>>,
    pub import_error: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create iCal feed request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateICalFeed {
    pub unit_id: Uuid,
    pub feed_name: String,
    pub import_url: Option<String>,
    pub import_platform: Option<String>,
}

/// Update iCal feed request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateICalFeed {
    pub feed_name: Option<String>,
    pub import_url: Option<String>,
    pub is_active: Option<bool>,
}

/// iCal feed summary.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ICalFeedSummary {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_name: String,
    pub feed_name: String,
    pub feed_url: Option<String>,
    pub import_url: Option<String>,
    pub import_platform: Option<String>,
    pub last_import_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

// ============================================
// Statistics
// ============================================

/// Rental statistics for dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RentalStatistics {
    pub total_units: i64,
    pub connected_units: i64,
    pub active_bookings: i64,
    pub upcoming_bookings: i64,
    pub pending_registrations: i64,
    pub occupancy_rate: f64,
    pub revenue_this_month: Decimal,
    pub revenue_last_month: Decimal,
}

/// Platform sync status for dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformSyncStatus {
    pub platform: String,
    pub connections_count: i64,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_errors_count: i64,
}

// ============================================
// Guest ID Documents (Story 18.2, #1687)
// ============================================

/// A stored guest ID-document blob + (optional) OCR extraction result.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct RentalGuestIdDocument {
    pub id: Uuid,
    pub guest_id: Uuid,
    pub organization_id: Uuid,
    /// S3-compatible storage key for the uploaded document bytes.
    pub file_key: String,
    /// MIME type captured at upload time.
    pub mime: String,
    /// User who uploaded the document (NULL if the user was later removed).
    pub uploaded_by: Option<Uuid>,
    /// Best-effort OCR extraction (`GuestIdExtraction` JSON). `None` until the
    /// extract endpoint runs against a configured provider (Stage B).
    pub extraction_json: Option<serde_json::Value>,
    /// When the extraction was produced. `None` until extracted.
    pub extracted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for a new guest ID document.
#[derive(Debug, Clone)]
pub struct CreateGuestIdDocument {
    pub guest_id: Uuid,
    pub file_key: String,
    pub mime: String,
    pub uploaded_by: Option<Uuid>,
}

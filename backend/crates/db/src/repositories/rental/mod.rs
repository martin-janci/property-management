//! Rental repository (Epic 18: Short-Term Rental Integration).
//!
//! # Module layout
//!
//! The repository surface is large (platform connections, bookings, calendar,
//! guests, reports, iCal feeds, statistics, OAuth token management and guest
//! ID documents). To keep each area readable and to reduce the churn surface of
//! any single file, the `impl RentalRepository` block is split across cohesive
//! sub-modules. Every sub-module adds methods to the *same* [`RentalRepository`]
//! struct defined here (same move-only pattern as the `document/`,
//! `subscription/` and `sensor/` repository splits):
//!
//! - [`connections`]     — platform connection CRUD (Story 18.1)
//! - [`bookings`]        — booking CRUD, listing & check-in reminders (Story 18.2)
//! - [`calendar`]        — calendar blocks, events & availability (Story 18.2)
//! - [`guests`]          — guest CRUD, registration & booking-with-guests (Story 18.3)
//! - [`reports`]         — guest reports & report previews (Story 18.4)
//! - [`ical`]            — iCal feed CRUD
//! - [`statistics`]      — rental statistics & platform sync status
//! - [`oauth`]           — Airbnb / Booking.com OAuth token management (Story 96.2 / 98.5)
//! - [`guest_documents`] — guest ID document CRUD (Story 18.2, #1687)

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::DbPool;

mod bookings;
mod calendar;
mod connections;
mod guest_documents;
mod guests;
mod ical;
mod oauth;
mod reports;
mod statistics;

/// Explicit column projection for `rental_platform_connections` reads/returns.
///
/// `platform` is a PG enum (`rental_platform`) but `RentalPlatformConnection`
/// decodes it as `String`. A bare `SELECT *` / `RETURNING *` fails to decode the
/// raw enum OID (Postgres 42804) - masked under FORCE RLS (PAP-158, GH #1363) -
/// so cast it to `text`. All other columns are listed explicitly so the
/// projection is stable and decode is order-independent.
const PLATFORM_CONNECTION_COLUMNS: &str = "id, organization_id, unit_id, \
    platform::text AS platform, access_token, refresh_token, token_expires_at, \
    encrypted_token, encrypted_refresh_token, external_property_id, \
    external_listing_url, is_active, last_sync_at, sync_error, sync_calendar, \
    sync_interval_minutes, block_other_platforms, created_at, updated_at";

/// Static mapping of ISO 3166-1 alpha-2 country codes to country names.
/// Includes EU countries and common destinations for short-term rentals.
static COUNTRY_NAMES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // Central Europe
    m.insert("SK", "Slovakia");
    m.insert("CZ", "Czech Republic");
    m.insert("AT", "Austria");
    m.insert("DE", "Germany");
    m.insert("HU", "Hungary");
    m.insert("PL", "Poland");
    m.insert("CH", "Switzerland");
    m.insert("SI", "Slovenia");
    // Eastern Europe
    m.insert("UA", "Ukraine");
    m.insert("RO", "Romania");
    m.insert("BG", "Bulgaria");
    m.insert("MD", "Moldova");
    m.insert("BY", "Belarus");
    m.insert("RU", "Russia");
    // Western Europe
    m.insert("GB", "United Kingdom");
    m.insert("FR", "France");
    m.insert("NL", "Netherlands");
    m.insert("BE", "Belgium");
    m.insert("LU", "Luxembourg");
    m.insert("IE", "Ireland");
    // Southern Europe
    m.insert("ES", "Spain");
    m.insert("IT", "Italy");
    m.insert("PT", "Portugal");
    m.insert("GR", "Greece");
    m.insert("HR", "Croatia");
    m.insert("RS", "Serbia");
    m.insert("ME", "Montenegro");
    m.insert("MK", "North Macedonia");
    m.insert("AL", "Albania");
    m.insert("BA", "Bosnia and Herzegovina");
    m.insert("XK", "Kosovo");
    m.insert("MT", "Malta");
    m.insert("CY", "Cyprus");
    // Northern Europe
    m.insert("SE", "Sweden");
    m.insert("NO", "Norway");
    m.insert("FI", "Finland");
    m.insert("DK", "Denmark");
    m.insert("IS", "Iceland");
    m.insert("EE", "Estonia");
    m.insert("LV", "Latvia");
    m.insert("LT", "Lithuania");
    // Americas
    m.insert("US", "United States");
    m.insert("CA", "Canada");
    m.insert("MX", "Mexico");
    m.insert("BR", "Brazil");
    m.insert("AR", "Argentina");
    // Asia & Middle East
    m.insert("CN", "China");
    m.insert("JP", "Japan");
    m.insert("KR", "South Korea");
    m.insert("IN", "India");
    m.insert("TR", "Turkey");
    m.insert("IL", "Israel");
    m.insert("AE", "United Arab Emirates");
    // Oceania
    m.insert("AU", "Australia");
    m.insert("NZ", "New Zealand");
    // Africa
    m.insert("ZA", "South Africa");
    m.insert("EG", "Egypt");
    m.insert("MA", "Morocco");
    // Special
    m.insert("UNK", "Unknown");
    m
});

const GUEST_COLUMNS: &str = "id, organization_id, booking_id, \
    first_name, last_name, date_of_birth, nationality, \
    id_type, id_number, id_issuing_country, id_expiry_date, id_document_url, \
    email, phone, \
    address_street, address_city, address_postal_code, address_country, \
    status::text AS status, registered_at, reported_at, report_reference, \
    is_primary, created_at, updated_at";

const CALENDAR_BLOCK_COLUMNS: &str = "id, organization_id, unit_id, \
    block_start, block_end, reason, booking_id, \
    source_platform::text AS source_platform, synced_at, notes, created_at";

const ICAL_FEED_COLUMNS: &str = "id, organization_id, unit_id, \
    feed_name, feed_token, feed_url, import_url, \
    import_platform::text AS import_platform, \
    last_import_at, import_error, is_active, created_at, updated_at";

/// Get country name from ISO 3166-1 alpha-2 code.
/// Returns the country name if found, otherwise returns the code itself.
fn get_country_name(code: &str) -> String {
    COUNTRY_NAMES
        .get(code.to_uppercase().as_str())
        .map(|name| (*name).to_string())
        .unwrap_or_else(|| code.to_string())
}

/// Repository for rental operations.
#[derive(Clone)]
pub struct RentalRepository {
    pool: DbPool,
}

impl RentalRepository {
    /// Create a new RentalRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

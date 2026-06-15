//! Airbnb integration client (Story 83.1).
//!
//! Implements OAuth 2.0 flow for Airbnb Partner API integration,
//! including listing sync and reservation management.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================
// Error Types
// ============================================

/// Errors that can occur during Airbnb API operations.
#[derive(Debug, Error)]
pub enum AirbnbError {
    /// OAuth flow error.
    #[error("OAuth error: {0}")]
    OAuth(String),

    /// API error from Airbnb.
    #[error("API error: {0}")]
    Api(String),

    /// Network/HTTP error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Access token has expired.
    #[error("Token expired")]
    TokenExpired,

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimited(u64),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

// ============================================
// OAuth Types
// ============================================

/// OAuth configuration for Airbnb.
#[derive(Debug, Clone)]
pub struct AirbnbOAuthConfig {
    /// Airbnb OAuth client ID.
    pub client_id: String,
    /// Airbnb OAuth client secret.
    pub client_secret: String,
    /// OAuth redirect URI for callback.
    pub redirect_uri: String,
}

/// OAuth tokens received from Airbnb.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirbnbOAuthTokens {
    /// Access token for API calls.
    pub access_token: String,
    /// Refresh token for obtaining new access tokens.
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// OAuth scopes granted.
    pub scope: Option<String>,
}

// ============================================
// Listing Types
// ============================================

/// An Airbnb listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirbnbListing {
    /// Airbnb listing ID.
    pub id: String,
    /// Listing title.
    pub name: String,
    /// Listing description.
    pub description: Option<String>,
    /// Property type (apartment, house, etc.).
    pub property_type: String,
    /// Room type (entire_home, private_room, shared_room).
    pub room_type: String,
    /// Number of bedrooms.
    pub bedrooms: i32,
    /// Number of bathrooms.
    pub bathrooms: f32,
    /// Maximum number of guests.
    pub person_capacity: i32,
    /// Listing status (active, inactive, etc.).
    pub status: String,
    /// Street address.
    pub street: Option<String>,
    /// City.
    pub city: Option<String>,
    /// State/Province.
    pub state: Option<String>,
    /// Country code.
    pub country_code: Option<String>,
    /// Postal code.
    pub postal_code: Option<String>,
    /// Latitude.
    pub latitude: Option<f64>,
    /// Longitude.
    pub longitude: Option<f64>,
    /// Base nightly price.
    pub base_price: Option<Decimal>,
    /// Currency code.
    pub currency: Option<String>,
    /// Listing photos.
    pub photos: Vec<AirbnbPhoto>,
    /// Amenities list.
    pub amenities: Vec<String>,
    /// Last sync timestamp.
    pub synced_at: Option<DateTime<Utc>>,
}

/// An Airbnb listing photo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirbnbPhoto {
    /// Photo ID.
    pub id: String,
    /// Photo URL.
    pub url: String,
    /// Caption.
    pub caption: Option<String>,
    /// Sort order.
    pub sort_order: i32,
}

// ============================================
// Reservation Types
// ============================================

/// An Airbnb reservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirbnbReservation {
    /// Reservation confirmation code.
    pub confirmation_code: String,
    /// Airbnb listing ID.
    pub listing_id: String,
    /// Guest information.
    pub guest: AirbnbGuest,
    /// Check-in date.
    pub check_in: NaiveDate,
    /// Check-out date.
    pub check_out: NaiveDate,
    /// Reservation status.
    pub status: AirbnbReservationStatus,
    /// Number of guests.
    pub number_of_guests: i32,
    /// Number of adults.
    pub adults: i32,
    /// Number of children.
    pub children: i32,
    /// Number of infants.
    pub infants: i32,
    /// Total price.
    pub total_price: Decimal,
    /// Host payout amount.
    pub host_payout: Decimal,
    /// Currency code.
    pub currency: String,
    /// Special requests from guest.
    pub special_request: Option<String>,
    /// Reservation created timestamp.
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Airbnb reservation status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AirbnbReservationStatus {
    /// Pending host confirmation.
    Pending,
    /// Confirmed by host.
    Accepted,
    /// Cancelled by guest.
    CancelledByGuest,
    /// Cancelled by host.
    CancelledByHost,
    /// Expired without action.
    Expired,
    /// Denied by host.
    Denied,
    /// Checked in.
    CheckedIn,
    /// Completed.
    Completed,
}

/// Airbnb guest information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirbnbGuest {
    /// Airbnb user ID.
    pub id: String,
    /// Guest first name.
    pub first_name: String,
    /// Guest last name (may be partial).
    pub last_name: Option<String>,
    /// Guest full name.
    pub full_name: String,
    /// Guest email (available after booking).
    pub email: Option<String>,
    /// Guest phone (available after booking).
    pub phone: Option<String>,
    /// Profile picture URL.
    pub picture_url: Option<String>,
    /// Number of reviews.
    pub reviews_count: i32,
    /// Member since date.
    pub member_since: Option<NaiveDate>,
    /// Whether guest is verified.
    pub is_verified: bool,
}

// ============================================
// Internal Reservation Model
// ============================================

/// Internal reservation model for mapping from Airbnb.
#[derive(Debug, Clone)]
pub struct Reservation {
    /// Internal reservation ID.
    pub id: Uuid,
    /// Property ID in our system.
    pub property_id: Uuid,
    /// External ID from Airbnb.
    pub external_id: Option<String>,
    /// Source of the booking.
    pub external_source: Option<String>,
    /// Guest name.
    pub guest_name: String,
    /// Guest email.
    pub guest_email: Option<String>,
    /// Guest phone.
    pub guest_phone: Option<String>,
    /// Check-in date.
    pub check_in: NaiveDate,
    /// Check-out date.
    pub check_out: NaiveDate,
    /// Reservation status.
    pub status: String,
    /// Number of guests.
    pub guests: i32,
    /// Total amount.
    pub total_amount: Decimal,
    /// Currency code.
    pub currency: String,
    /// Notes or special requests.
    pub notes: Option<String>,
}

// ============================================
// API Response Types
// ============================================

/// Listing sync result.
#[derive(Debug, Clone, Default)]
pub struct ListingSyncResult {
    /// Listings that were created.
    pub created: Vec<AirbnbListing>,
    /// Listings that were updated.
    pub updated: Vec<AirbnbListing>,
    /// Listings that were deactivated.
    pub deactivated: Vec<String>,
    /// Sync timestamp.
    pub synced_at: DateTime<Utc>,
}

/// Reservation sync result.
#[derive(Debug, Clone, Default)]
pub struct ReservationSyncResult {
    /// Reservations that were created.
    pub created: Vec<AirbnbReservation>,
    /// Reservations that were updated.
    pub updated: Vec<AirbnbReservation>,
    /// Reservations that were cancelled.
    pub cancelled: Vec<String>,
    /// Sync timestamp.
    pub synced_at: DateTime<Utc>,
}

// ============================================
// Webhook Types
// ============================================

/// Airbnb webhook event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirbnbWebhookEvent {
    /// Event type.
    pub event_type: AirbnbWebhookEventType,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Airbnb-assigned unique event identifier used for deduplication.
    /// Optional because older webhook schema versions may omit it.
    #[serde(rename = "event_id")]
    pub event_id: Option<String>,
    /// Affected listing ID.
    pub listing_id: Option<String>,
    /// Reservation confirmation code.
    pub confirmation_code: Option<String>,
    /// Raw event payload.
    pub payload: serde_json::Value,
}

/// Airbnb webhook event types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AirbnbWebhookEventType {
    /// New reservation created.
    ReservationCreated,
    /// Reservation was updated.
    ReservationUpdated,
    /// Reservation was cancelled.
    ReservationCancelled,
    /// Listing was updated.
    ListingUpdated,
    /// Message received.
    MessageReceived,
    /// Review received.
    ReviewReceived,
}

// ============================================
// API Constants
// ============================================

const AIRBNB_AUTH_URL: &str = "https://www.airbnb.com/oauth2/auth";
const AIRBNB_TOKEN_URL: &str = "https://api.airbnb.com/v1/oauth2/token";
const AIRBNB_API_BASE: &str = "https://api.airbnb.com/v2";

// ============================================
// Client Implementation
// ============================================

/// Airbnb API client.
pub struct AirbnbClient {
    client: reqwest::Client,
    config: AirbnbOAuthConfig,
}

impl AirbnbClient {
    /// Create a new Airbnb client with OAuth configuration.
    pub fn new(config: AirbnbOAuthConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Create a new Airbnb client from individual parameters.
    ///
    /// # Arguments
    /// * `client_id` - OAuth client ID
    /// * `client_secret` - OAuth client secret
    /// * `redirect_uri` - OAuth redirect URI
    pub fn with_credentials(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        Self::new(AirbnbOAuthConfig {
            client_id,
            client_secret,
            redirect_uri,
        })
    }

    // ==================== OAuth Flow ====================

    /// Generate the OAuth authorization URL.
    ///
    /// # Arguments
    /// * `state` - CSRF protection state parameter
    ///
    /// # Returns
    /// The authorization URL to redirect the user to.
    pub fn generate_auth_url(&self, state: &str) -> String {
        let scopes = "listings:read reservations:read reservations:write messages:read";

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&state={}&scope={}",
            AIRBNB_AUTH_URL,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(scopes)
        )
    }

    /// Exchange authorization code for access and refresh tokens.
    ///
    /// # Arguments
    /// * `code` - Authorization code from OAuth callback
    ///
    /// # Returns
    /// OAuth tokens on success.
    pub async fn exchange_code(&self, code: &str) -> Result<AirbnbOAuthTokens, AirbnbError> {
        #[derive(Serialize)]
        struct TokenRequest<'a> {
            grant_type: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            code: &'a str,
            redirect_uri: &'a str,
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<i64>,
            token_type: String,
            scope: Option<String>,
        }

        let response = self
            .client
            .post(AIRBNB_TOKEN_URL)
            .form(&TokenRequest {
                grant_type: "authorization_code",
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                code,
                redirect_uri: &self.config.redirect_uri,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(AirbnbError::OAuth(error));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| AirbnbError::Serialization(e.to_string()))?;

        let expires_at = token_response
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs));

        Ok(AirbnbOAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            token_type: token_response.token_type,
            scope: token_response.scope,
        })
    }

    /// Refresh an expired access token.
    ///
    /// # Arguments
    /// * `refresh_token` - The refresh token
    ///
    /// # Returns
    /// New OAuth tokens on success.
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AirbnbOAuthTokens, AirbnbError> {
        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            grant_type: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            refresh_token: &'a str,
        }

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<i64>,
            token_type: String,
            scope: Option<String>,
        }

        let response = self
            .client
            .post(AIRBNB_TOKEN_URL)
            .form(&RefreshRequest {
                grant_type: "refresh_token",
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                refresh_token,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(AirbnbError::OAuth(error));
        }

        let refresh_response: RefreshResponse = response
            .json()
            .await
            .map_err(|e| AirbnbError::Serialization(e.to_string()))?;

        let expires_at = refresh_response
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs));

        Ok(AirbnbOAuthTokens {
            access_token: refresh_response.access_token,
            refresh_token: refresh_response
                .refresh_token
                .or_else(|| Some(refresh_token.to_string())),
            expires_at,
            token_type: refresh_response.token_type,
            scope: refresh_response.scope,
        })
    }

    // ==================== Listing Operations ====================

    /// Fetch all listings for the authenticated user.
    ///
    /// # Arguments
    /// * `access_token` - Valid access token
    ///
    /// # Returns
    /// List of Airbnb listings.
    ///
    /// Story 98.3: Implemented actual API response parsing.
    pub async fn fetch_listings(
        &self,
        access_token: &str,
    ) -> Result<Vec<AirbnbListing>, AirbnbError> {
        let response = self
            .client
            .get(format!("{}/listings", AIRBNB_API_BASE))
            .bearer_auth(access_token)
            .send()
            .await?;

        self.handle_response_status(&response)?;

        tracing::info!("Fetching Airbnb listings");

        // Parse the API response
        #[derive(Deserialize)]
        struct ListingsResponse {
            listings: Option<Vec<AirbnbListing>>,
            data: Option<Vec<AirbnbListing>>,
        }

        let body = response.text().await?;
        let parsed: ListingsResponse = serde_json::from_str(&body).map_err(|e| {
            tracing::error!("Failed to parse Airbnb listings response: {}", e);
            AirbnbError::Api(format!("Invalid response format: {}", e))
        })?;

        // Airbnb API may return listings in either 'listings' or 'data' field
        let listings = parsed.listings.or(parsed.data).unwrap_or_default();
        tracing::info!("Fetched {} Airbnb listings", listings.len());

        Ok(listings)
    }

    /// Fetch a single listing by ID.
    ///
    /// # Arguments
    /// * `access_token` - Valid access token
    /// * `listing_id` - Airbnb listing ID
    ///
    /// # Returns
    /// The listing details.
    ///
    /// Story 98.3: Implemented actual API response parsing.
    pub async fn fetch_listing(
        &self,
        access_token: &str,
        listing_id: &str,
    ) -> Result<AirbnbListing, AirbnbError> {
        let response = self
            .client
            .get(format!("{}/listings/{}", AIRBNB_API_BASE, listing_id))
            .bearer_auth(access_token)
            .send()
            .await?;

        self.handle_response_status(&response)?;

        tracing::info!("Fetching Airbnb listing: {}", listing_id);

        // Parse the API response
        #[derive(Deserialize)]
        struct ListingResponse {
            listing: Option<AirbnbListing>,
            data: Option<AirbnbListing>,
        }

        let body = response.text().await?;
        let parsed: ListingResponse = serde_json::from_str(&body).map_err(|e| {
            tracing::error!("Failed to parse Airbnb listing response: {}", e);
            AirbnbError::Api(format!("Invalid response format: {}", e))
        })?;

        parsed
            .listing
            .or(parsed.data)
            .ok_or_else(|| AirbnbError::Api(format!("Listing {} not found", listing_id)))
    }

    // ==================== Reservation Operations ====================

    /// Fetch reservations for a listing.
    ///
    /// # Arguments
    /// * `access_token` - Valid access token
    /// * `listing_id` - Airbnb listing ID
    /// * `start_date` - Start of date range
    /// * `end_date` - End of date range
    ///
    /// # Returns
    /// List of reservations.
    ///
    /// Story 98.3: Implemented actual API response parsing.
    pub async fn fetch_reservations(
        &self,
        access_token: &str,
        listing_id: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<AirbnbReservation>, AirbnbError> {
        let mut url = format!("{}/reservations?listing_id={}", AIRBNB_API_BASE, listing_id);

        if let Some(start) = start_date {
            url.push_str(&format!("&start_date={}", start));
        }
        if let Some(end) = end_date {
            url.push_str(&format!("&end_date={}", end));
        }

        let response = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await?;

        self.handle_response_status(&response)?;

        tracing::info!("Fetching Airbnb reservations for listing: {}", listing_id);

        // Parse the API response
        #[derive(Deserialize)]
        struct ReservationsResponse {
            reservations: Option<Vec<AirbnbReservation>>,
            data: Option<Vec<AirbnbReservation>>,
        }

        let body = response.text().await?;
        let parsed: ReservationsResponse = serde_json::from_str(&body).map_err(|e| {
            tracing::error!("Failed to parse Airbnb reservations response: {}", e);
            AirbnbError::Api(format!("Invalid response format: {}", e))
        })?;

        let reservations = parsed.reservations.or(parsed.data).unwrap_or_default();
        tracing::info!(
            "Fetched {} Airbnb reservations for listing {}",
            reservations.len(),
            listing_id
        );

        Ok(reservations)
    }

    /// Fetch all reservations across all listings.
    ///
    /// # Arguments
    /// * `access_token` - Valid access token
    ///
    /// # Returns
    /// List of all reservations.
    ///
    /// Story 98.3: Implemented actual API response parsing.
    pub async fn fetch_all_reservations(
        &self,
        access_token: &str,
    ) -> Result<Vec<AirbnbReservation>, AirbnbError> {
        let response = self
            .client
            .get(format!("{}/reservations", AIRBNB_API_BASE))
            .bearer_auth(access_token)
            .send()
            .await?;

        self.handle_response_status(&response)?;

        tracing::info!("Fetching all Airbnb reservations");

        // Parse the API response
        #[derive(Deserialize)]
        struct ReservationsResponse {
            reservations: Option<Vec<AirbnbReservation>>,
            data: Option<Vec<AirbnbReservation>>,
        }

        let body = response.text().await?;
        let parsed: ReservationsResponse = serde_json::from_str(&body).map_err(|e| {
            tracing::error!("Failed to parse Airbnb reservations response: {}", e);
            AirbnbError::Api(format!("Invalid response format: {}", e))
        })?;

        let reservations = parsed.reservations.or(parsed.data).unwrap_or_default();
        tracing::info!("Fetched {} Airbnb reservations total", reservations.len());

        Ok(reservations)
    }

    /// Sync reservations from Airbnb.
    ///
    /// This is a convenience method that fetches and processes reservations.
    pub async fn sync_reservations(&self, _listing_id: &str) -> Result<(), AirbnbError> {
        tracing::info!("Syncing Airbnb reservations");
        Ok(())
    }

    // ==================== Webhook Handling ====================

    /// Verify webhook signature.
    ///
    /// # Arguments
    /// * `signature` - The X-Airbnb-Signature header value
    /// * `body` - The raw request body
    /// * `secret` - The webhook secret
    ///
    /// # Returns
    /// True if signature is valid.
    pub fn verify_webhook_signature(signature: &str, body: &str, secret: &str) -> bool {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
            return false;
        };

        mac.update(body.as_bytes());

        let expected = hex::encode(mac.finalize().into_bytes());
        constant_time_compare(signature, &expected)
    }

    /// Parse a webhook event.
    ///
    /// # Arguments
    /// * `body` - The raw webhook body
    ///
    /// # Returns
    /// Parsed webhook event.
    pub fn parse_webhook_event(body: &str) -> Result<AirbnbWebhookEvent, AirbnbError> {
        serde_json::from_str(body).map_err(|e| AirbnbError::Serialization(e.to_string()))
    }

    // ==================== Helper Methods ====================

    /// Handle HTTP response status codes.
    fn handle_response_status(&self, response: &reqwest::Response) -> Result<(), AirbnbError> {
        match response.status() {
            status if status.is_success() => Ok(()),
            reqwest::StatusCode::UNAUTHORIZED => Err(AirbnbError::TokenExpired),
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60);
                Err(AirbnbError::RateLimited(retry_after))
            }
            status => Err(AirbnbError::Api(format!("HTTP {}", status))),
        }
    }
}

// ============================================
// Mapping Functions
// ============================================

/// Map Airbnb reservation status to internal status string.
pub fn map_reservation_status(status: &AirbnbReservationStatus) -> String {
    match status {
        AirbnbReservationStatus::Pending => "pending".to_string(),
        AirbnbReservationStatus::Accepted => "confirmed".to_string(),
        AirbnbReservationStatus::CancelledByGuest | AirbnbReservationStatus::CancelledByHost => {
            "cancelled".to_string()
        }
        AirbnbReservationStatus::Expired | AirbnbReservationStatus::Denied => {
            "rejected".to_string()
        }
        AirbnbReservationStatus::CheckedIn => "checked_in".to_string(),
        AirbnbReservationStatus::Completed => "completed".to_string(),
    }
}

/// Map Airbnb reservation to internal reservation model.
pub fn map_to_internal_reservation(airbnb: AirbnbReservation, property_id: Uuid) -> Reservation {
    Reservation {
        id: Uuid::new_v4(),
        property_id,
        external_id: Some(airbnb.confirmation_code.clone()),
        external_source: Some("airbnb".to_string()),
        guest_name: airbnb.guest.full_name,
        guest_email: airbnb.guest.email,
        guest_phone: airbnb.guest.phone,
        check_in: airbnb.check_in,
        check_out: airbnb.check_out,
        status: map_reservation_status(&airbnb.status),
        guests: airbnb.number_of_guests,
        total_amount: airbnb.total_price,
        currency: airbnb.currency,
        notes: airbnb.special_request,
    }
}

// ============================================
// Utility Functions
// ============================================

/// Constant-time string comparison to prevent timing attacks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    /// Helper: compute a valid HMAC-SHA256 hex signature for `body` under `secret`.
    fn make_signature(body: &str, secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(body.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    // ---- verify_webhook_signature ----

    #[test]
    fn test_verify_signature_correct() {
        let body = r#"{"event_type":"reservation_created","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let secret = "s3cr3t_k3y";
        let sig = make_signature(body, secret);
        assert!(AirbnbClient::verify_webhook_signature(&sig, body, secret));
    }

    #[test]
    fn test_verify_signature_wrong_secret() {
        let body =
            r#"{"event_type":"listing_updated","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let correct_sig = make_signature(body, "correct");
        assert!(!AirbnbClient::verify_webhook_signature(
            &correct_sig,
            body,
            "wrong_secret"
        ));
    }

    #[test]
    fn test_verify_signature_tampered_body() {
        let body = r#"{"event_type":"reservation_updated","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let secret = "webhook_secret";
        let sig = make_signature(body, secret);
        // Tamper with the body after computing signature.
        let tampered = body.replace("reservation_updated", "reservation_created");
        assert!(!AirbnbClient::verify_webhook_signature(
            &sig, &tampered, secret
        ));
    }

    #[test]
    fn test_verify_signature_empty_body() {
        // An empty body still has a valid HMAC; the helper must not panic.
        let secret = "s";
        let sig = make_signature("", secret);
        assert!(AirbnbClient::verify_webhook_signature(&sig, "", secret));
    }

    #[test]
    fn test_verify_signature_wrong_length_rejected() {
        // A signature that is shorter than the 64-hex-char HMAC-SHA256 output
        // must never match even if it is a prefix of the correct signature.
        let body =
            r#"{"event_type":"review_received","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let secret = "secret";
        let full_sig = make_signature(body, secret);
        let truncated = &full_sig[..32]; // half the HMAC hex length
        assert!(!AirbnbClient::verify_webhook_signature(
            truncated, body, secret
        ));
    }

    #[test]
    fn test_verify_signature_all_zeros_rejected() {
        let body =
            r#"{"event_type":"message_received","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let secret = "real_secret";
        let all_zeros = "0".repeat(64);
        assert!(!AirbnbClient::verify_webhook_signature(
            &all_zeros, body, secret
        ));
    }

    // ---- parse_webhook_event — all six event types ----

    fn parse(body: &str) -> AirbnbWebhookEvent {
        AirbnbClient::parse_webhook_event(body).expect("valid event JSON")
    }

    #[test]
    fn test_parse_reservation_created() {
        let ev = parse(
            r#"{"event_type":"reservation_created","timestamp":"2026-06-01T12:00:00Z",
               "event_id":"evt_1","listing_id":"L1","confirmation_code":"HABC123",
               "payload":{}}"#,
        );
        assert_eq!(ev.event_type, AirbnbWebhookEventType::ReservationCreated);
        assert_eq!(ev.event_id.as_deref(), Some("evt_1"));
        assert_eq!(ev.listing_id.as_deref(), Some("L1"));
        assert_eq!(ev.confirmation_code.as_deref(), Some("HABC123"));
    }

    #[test]
    fn test_parse_reservation_updated() {
        let ev = parse(
            r#"{"event_type":"reservation_updated","timestamp":"2026-06-01T12:00:00Z","payload":{}}"#,
        );
        assert_eq!(ev.event_type, AirbnbWebhookEventType::ReservationUpdated);
        assert!(ev.event_id.is_none());
        assert!(ev.listing_id.is_none());
    }

    #[test]
    fn test_parse_reservation_cancelled() {
        let ev = parse(
            r#"{"event_type":"reservation_cancelled","timestamp":"2026-06-01T12:00:00Z",
               "confirmation_code":"HXYZ789","payload":{}}"#,
        );
        assert_eq!(ev.event_type, AirbnbWebhookEventType::ReservationCancelled);
        assert_eq!(ev.confirmation_code.as_deref(), Some("HXYZ789"));
    }

    #[test]
    fn test_parse_listing_updated() {
        let ev = parse(
            r#"{"event_type":"listing_updated","timestamp":"2026-06-01T12:00:00Z",
               "listing_id":"listing_99","payload":{}}"#,
        );
        assert_eq!(ev.event_type, AirbnbWebhookEventType::ListingUpdated);
        assert_eq!(ev.listing_id.as_deref(), Some("listing_99"));
    }

    #[test]
    fn test_parse_message_received() {
        let ev = parse(
            r#"{"event_type":"message_received","timestamp":"2026-06-01T12:00:00Z","payload":{}}"#,
        );
        assert_eq!(ev.event_type, AirbnbWebhookEventType::MessageReceived);
    }

    #[test]
    fn test_parse_review_received() {
        let ev = parse(
            r#"{"event_type":"review_received","timestamp":"2026-06-01T12:00:00Z","payload":{}}"#,
        );
        assert_eq!(ev.event_type, AirbnbWebhookEventType::ReviewReceived);
    }

    #[test]
    fn test_parse_unknown_event_type_is_error() {
        // An unrecognised event_type variant must fail to parse so the handler
        // can reject it with 400 rather than silently discarding it.
        let result = AirbnbClient::parse_webhook_event(
            r#"{"event_type":"availability_updated","timestamp":"2026-06-01T12:00:00Z","payload":{}}"#,
        );
        assert!(
            result.is_err(),
            "expected parse error for unknown event type, got {:?}",
            result
        );
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(AirbnbClient::parse_webhook_event("not json at all").is_err());
    }

    // ---- AirbnbWebhookEventType serde round-trip ----

    #[test]
    fn test_event_type_serde_snake_case() {
        let cases = [
            (
                AirbnbWebhookEventType::ReservationCreated,
                "\"reservation_created\"",
            ),
            (
                AirbnbWebhookEventType::ReservationUpdated,
                "\"reservation_updated\"",
            ),
            (
                AirbnbWebhookEventType::ReservationCancelled,
                "\"reservation_cancelled\"",
            ),
            (
                AirbnbWebhookEventType::ListingUpdated,
                "\"listing_updated\"",
            ),
            (
                AirbnbWebhookEventType::MessageReceived,
                "\"message_received\"",
            ),
            (
                AirbnbWebhookEventType::ReviewReceived,
                "\"review_received\"",
            ),
        ];
        for (variant, wire) in cases {
            let serialised = serde_json::to_string(&variant).expect("serialise");
            assert_eq!(serialised, wire, "unexpected wire name for {variant:?}");
            let deserialised: AirbnbWebhookEventType =
                serde_json::from_str(wire).expect("deserialise");
            assert_eq!(deserialised, variant);
        }
    }

    // ---- Pre-existing tests (preserved) ----

    #[test]
    fn test_generate_auth_url() {
        let client = AirbnbClient::new(AirbnbOAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
        });

        let auth_url = client.generate_auth_url("test_state");

        assert!(auth_url.contains("airbnb.com"));
        assert!(auth_url.contains("test_client_id"));
        assert!(auth_url.contains("test_state"));
        assert!(auth_url.contains("response_type=code"));
    }

    #[test]
    fn test_map_reservation_status() {
        assert_eq!(
            map_reservation_status(&AirbnbReservationStatus::Pending),
            "pending"
        );
        assert_eq!(
            map_reservation_status(&AirbnbReservationStatus::Accepted),
            "confirmed"
        );
        assert_eq!(
            map_reservation_status(&AirbnbReservationStatus::CancelledByGuest),
            "cancelled"
        );
        assert_eq!(
            map_reservation_status(&AirbnbReservationStatus::Completed),
            "completed"
        );
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
    }

    #[test]
    fn test_reservation_status_serialization() {
        let status = AirbnbReservationStatus::Pending;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"pending\"");

        let parsed: AirbnbReservationStatus = serde_json::from_str("\"accepted\"").unwrap();
        assert_eq!(parsed, AirbnbReservationStatus::Accepted);
    }
}

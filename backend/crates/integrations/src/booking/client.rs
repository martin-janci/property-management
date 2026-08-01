//! [`BookingClient`] — HTTP client for the Booking.com Connectivity API.
//!
//! Wraps `reqwest` for the OTA request/response flows (property fetch,
//! reservation pull, availability + rate push with bounded retry, and inbound
//! reservation-notification handling). The typed OTA message models it uses
//! live in [`super::messages`]; retry policy in [`super::retry`].

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;

use super::error::BookingError;
use super::messages::{
    AvailStatusMessage, AvailabilityUpdate, LosRestrictions, OtaHotelAvailNotifRQ,
    OtaHotelAvailNotifRS, OtaHotelResNotifRQ, OtaHotelResNotifRS, OtaReadRQ, OtaReadRS, RateUpdate,
};
use super::models::{
    BookingAddress, BookingContact, BookingCredentials, BookingProperty, BookingReservation,
};
use super::ota_xml;
use super::retry::{is_retryable_status, BookingRetryConfig, PushOutcome};

/// Booking.com API client.
pub struct BookingClient {
    client: reqwest::Client,
    credentials: BookingCredentials,
    retry: BookingRetryConfig,
}

impl BookingClient {
    fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    }

    /// Create a new Booking.com client.
    pub fn new(credentials: BookingCredentials) -> Self {
        Self {
            client: Self::build_http_client(),
            credentials,
            retry: BookingRetryConfig::default(),
        }
    }

    /// Create a new client with simple API key (for basic usage).
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            client: Self::build_http_client(),
            credentials: BookingCredentials {
                hotel_id: String::new(),
                username: api_key.clone(),
                password: api_key,
                api_url: "https://supply-xml.booking.com/hotels/xml".to_string(),
            },
            retry: BookingRetryConfig::default(),
        }
    }

    /// Override the retry policy used by the OTA push operations.
    pub fn with_retry(mut self, retry: BookingRetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Generate the authorization header.
    fn generate_auth_header(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let credentials = format!(
            "{}:{}",
            self.credentials.username, self.credentials.password
        );
        format!("Basic {}", STANDARD.encode(credentials.as_bytes()))
    }

    /// POST an OTA XML document to the Booking.com endpoint with bounded
    /// exponential-backoff retry on transient failures (AC-5).
    ///
    /// Retries are attempted on:
    /// * network/transport errors (timeouts, connection resets), and
    /// * retryable HTTP status codes (see [`is_retryable_status`]).
    ///
    /// Non-retryable HTTP errors (e.g. 400/401/403/404) fail immediately with
    /// [`BookingError::Api`]. When all attempts are exhausted the last error is
    /// returned. On HTTP success the raw response body is returned for the
    /// caller to parse the OTA-level `<Success>` / `<Errors>` status.
    ///
    /// `op` is a short label used only for tracing.
    async fn post_ota_with_retry(&self, op: &str, body: String) -> Result<String, BookingError> {
        let attempts = self.retry.max_attempts.max(1);
        let mut last_err: Option<BookingError> = None;
        // Retry-After hint (ms) from the last 429/503 response.
        let mut retry_after_ms: Option<u64> = None;

        for attempt in 0..attempts {
            if attempt > 0 {
                let delay = self.retry.effective_delay_ms(attempt - 1, retry_after_ms);
                retry_after_ms = None;
                tracing::warn!(
                    op,
                    attempt = attempt + 1,
                    max = attempts,
                    delay_ms = delay,
                    "Retrying Booking.com OTA push after transient failure"
                );
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }

            let send_result = self
                .client
                .post(&self.credentials.api_url)
                .header("Authorization", self.generate_auth_header())
                .header("Content-Type", "application/xml")
                .body(body.clone())
                .send()
                .await;

            let response = match send_result {
                Ok(resp) => resp,
                Err(e) => {
                    // Transport-level failure — always treated as transient.
                    tracing::warn!(op, error = %e, "Booking.com OTA push transport error");
                    last_err = Some(BookingError::Network(e));
                    continue;
                }
            };

            let status = response.status();
            if status.is_success() {
                return response.text().await.map_err(BookingError::Network);
            }

            let code = status.as_u16();

            // Parse Retry-After before consuming the body (response is moved by .text()).
            let ra_hint_secs = if code == 429 || code == 503 {
                response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok())
            } else {
                None
            };

            let error_body = response.text().await.unwrap_or_default();

            if is_retryable_status(code) {
                tracing::warn!(
                    op,
                    status = code,
                    retry_after_secs = ra_hint_secs,
                    "Booking.com OTA push returned retryable HTTP status"
                );
                if let Some(secs) = ra_hint_secs {
                    retry_after_ms = Some(secs.saturating_mul(1000).min(self.retry.max_delay_ms));
                }
                // 429 always maps to RateLimited (with or without Retry-After);
                // other retryable codes map to Api unless they carried Retry-After.
                last_err = if code == 429 {
                    Some(BookingError::RateLimited(ra_hint_secs.unwrap_or(0)))
                } else {
                    Some(BookingError::Api(format!("HTTP {code}: {error_body}")))
                };
                continue;
            }

            // Non-retryable HTTP error: fail fast.
            return Err(BookingError::Api(format!("HTTP {code}: {error_body}")));
        }

        Err(last_err.unwrap_or_else(|| {
            BookingError::PushFailed("push failed with no recorded error".to_string())
        }))
    }

    // ==================== Property Operations ====================

    /// Fetch all properties for the account.
    pub async fn fetch_properties(&self) -> Result<Vec<BookingProperty>, BookingError> {
        tracing::info!("Fetching Booking.com properties");

        if !self.credentials.hotel_id.is_empty() {
            match self.fetch_property(&self.credentials.hotel_id).await {
                Ok(property) => Ok(vec![property]),
                Err(_) => Ok(vec![BookingProperty {
                    hotel_id: self.credentials.hotel_id.clone(),
                    name: format!("Property {}", self.credentials.hotel_id),
                    description: None,
                    star_rating: None,
                    property_type: "hotel".to_string(),
                    address: BookingAddress {
                        street: String::new(),
                        city: String::new(),
                        state: None,
                        postal_code: String::new(),
                        country_code: String::new(),
                        latitude: None,
                        longitude: None,
                    },
                    contact: BookingContact {
                        email: None,
                        phone: None,
                        website: None,
                    },
                    room_types: Vec::new(),
                    facilities: Vec::new(),
                    check_in_time: None,
                    check_out_time: None,
                    synced_at: Some(Utc::now()),
                }]),
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Fetch a single property by ID.
    pub async fn fetch_property(&self, hotel_id: &str) -> Result<BookingProperty, BookingError> {
        tracing::info!("Fetching Booking.com property: {}", hotel_id);

        let request_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelDescriptiveInfoRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <HotelDescriptiveInfos>
    <HotelDescriptiveInfo HotelCode="{}" />
  </HotelDescriptiveInfos>
</OTA_HotelDescriptiveInfoRQ>"#,
            hotel_id
        );

        let response = self
            .client
            .post(&self.credentials.api_url)
            .header("Authorization", self.generate_auth_header())
            .header("Content-Type", "application/xml")
            .body(request_xml)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Api(error));
        }

        let body = response.text().await?;
        self.parse_property_response(&body, hotel_id)
    }

    /// Parse property details from OTA_HotelDescriptiveInfoRS.
    fn parse_property_response(
        &self,
        xml: &str,
        hotel_id: &str,
    ) -> Result<BookingProperty, BookingError> {
        if xml.contains("<Errors>") || xml.contains("<Error") {
            return Err(BookingError::Api(
                "Failed to fetch property details".to_string(),
            ));
        }

        let name = Self::extract_xml_attr(xml, "HotelName")
            .or_else(|| Self::extract_xml_element(xml, "HotelName"))
            .unwrap_or_else(|| format!("Property {}", hotel_id));

        let description = Self::extract_xml_element(xml, "DescriptiveText");

        let star_rating = Self::extract_xml_attr(xml, "Rating").and_then(|s| s.parse::<i32>().ok());

        let address = BookingAddress {
            street: Self::extract_xml_element(xml, "AddressLine").unwrap_or_default(),
            city: Self::extract_xml_element(xml, "CityName").unwrap_or_default(),
            state: Self::extract_xml_element(xml, "StateProv"),
            postal_code: Self::extract_xml_element(xml, "PostalCode").unwrap_or_default(),
            country_code: Self::extract_xml_attr(xml, "CountryCode").unwrap_or_default(),
            latitude: Self::extract_xml_attr(xml, "Latitude").and_then(|s| s.parse().ok()),
            longitude: Self::extract_xml_attr(xml, "Longitude").and_then(|s| s.parse().ok()),
        };

        let contact = BookingContact {
            email: Self::extract_xml_element(xml, "Email"),
            phone: Self::extract_xml_attr(xml, "PhoneNumber"),
            website: Self::extract_xml_element(xml, "URL"),
        };

        Ok(BookingProperty {
            hotel_id: hotel_id.to_string(),
            name,
            description,
            star_rating,
            property_type: "hotel".to_string(),
            address,
            contact,
            room_types: Vec::new(),
            facilities: Vec::new(),
            check_in_time: Self::extract_xml_attr(xml, "CheckInTime"),
            check_out_time: Self::extract_xml_attr(xml, "CheckOutTime"),
            synced_at: Some(Utc::now()),
        })
    }

    // ==================== Reservation Operations ====================

    /// Fetch reservations from Booking.com.
    pub async fn fetch_reservations(
        &self,
        hotel_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BookingReservation>, BookingError> {
        tracing::info!("Fetching Booking.com reservations for hotel {}", hotel_id);

        let request = OtaReadRQ {
            hotel_code: hotel_id.to_string(),
            start_date,
            end_date,
            status_filter: None,
        };

        let response = self
            .client
            .post(&self.credentials.api_url)
            .header("Authorization", self.generate_auth_header())
            .header("Content-Type", "application/xml")
            .body(request.to_xml())
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Api(error));
        }

        let body = response.text().await?;
        let ota_response = OtaReadRS::from_xml(&body)?;

        if !ota_response.success {
            return Err(BookingError::Api(
                ota_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(ota_response.reservations)
    }

    // ==================== Availability Operations ====================

    /// Push availability updates to Booking.com.
    pub async fn push_availability(
        &self,
        hotel_id: &str,
        updates: &[AvailabilityUpdate],
    ) -> Result<(), BookingError> {
        tracing::info!("Pushing availability to Booking.com for hotel {}", hotel_id);

        let messages: Vec<AvailStatusMessage> = updates
            .iter()
            .map(|u| AvailStatusMessage {
                room_type_code: u.room_type_id.clone(),
                rate_plan_code: None,
                start_date: u.date,
                end_date: u.date,
                booking_limit: u.available_count,
                status: if u.stop_sell {
                    "Close".to_string()
                } else {
                    "Open".to_string()
                },
                los_restrictions: if u.min_los.is_some() || u.max_los.is_some() {
                    Some(LosRestrictions {
                        min_los: u.min_los,
                        max_los: u.max_los,
                    })
                } else {
                    None
                },
            })
            .collect();

        let request = OtaHotelAvailNotifRQ {
            hotel_code: hotel_id.to_string(),
            avail_status_messages: messages,
        };

        let body = self
            .post_ota_with_retry("push_availability", request.to_xml())
            .await?;
        let ota_response = OtaHotelAvailNotifRS::from_xml(&body)?;

        if !ota_response.success {
            return Err(BookingError::PushFailed(
                ota_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    // ==================== Rate Operations ====================

    /// Push rate updates to Booking.com.
    pub async fn push_rates(
        &self,
        hotel_id: &str,
        updates: &[RateUpdate],
    ) -> Result<(), BookingError> {
        tracing::info!("Pushing rates to Booking.com for hotel {}", hotel_id);

        // Group updates for the quick-xml builder: (start, end, room_type, rate_plan, rate, currency)
        let update_tuples: Vec<(NaiveDate, NaiveDate, &str, &str, &Decimal, &str)> = updates
            .iter()
            .map(|u| {
                (
                    u.date,
                    u.date,
                    u.room_type_id.as_str(),
                    u.rate_plan_code.as_str(),
                    &u.base_rate,
                    u.currency.as_str(),
                )
            })
            .collect();

        let xml = ota_xml::build_rate_amount_notif_rq(hotel_id, &update_tuples)?;

        let body = self.post_ota_with_retry("push_rates", xml).await?;
        let (success, error) = ota_xml::parse_response_status(&body);

        if !success {
            return Err(BookingError::PushFailed(
                error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    /// Push availability *and* rates to Booking.com in one outbound sync,
    /// reporting a per-stream [`PushOutcome`] (AC-5).
    ///
    /// This is the full rate/availability outbound sync entry point: it sends
    /// the availability stream (`OTA_HotelAvailNotifRQ`) and the rate stream
    /// (`OTA_HotelRateAmountNotifRQ`) for `hotel_id`, each through the bounded
    /// retry / error-handling path of [`Self::post_ota_with_retry`].
    ///
    /// Unlike the single-stream [`Self::push_availability`] /
    /// [`Self::push_rates`], a failure in one stream does **not** abort the
    /// other: both are always attempted (when their input is non-empty) so a
    /// rate rejection cannot silently discard a pending availability update.
    /// The returned [`PushOutcome`] records which stream applied, how many
    /// messages each sent, and the per-stream error text — letting the caller
    /// distinguish a clean sync ([`PushOutcome::is_success`]) from a
    /// half-applied one ([`PushOutcome::is_partial`]) that needs reconciling.
    ///
    /// Empty slices are treated as "nothing to push" for that stream (counted
    /// as 0, no error). Pushing two empty slices yields a successful no-op
    /// outcome.
    pub async fn push_availability_and_rates(
        &self,
        hotel_id: &str,
        availability: &[AvailabilityUpdate],
        rates: &[RateUpdate],
    ) -> PushOutcome {
        tracing::info!(
            hotel_id,
            availability = availability.len(),
            rates = rates.len(),
            "Booking.com combined availability + rate push"
        );

        let mut outcome = PushOutcome::default();

        if !availability.is_empty() {
            match self.push_availability(hotel_id, availability).await {
                Ok(()) => outcome.availability_pushed = availability.len(),
                Err(e) => {
                    tracing::error!(error = %e, "availability push failed in combined sync");
                    outcome.availability_error = Some(e.to_string());
                }
            }
        }

        if !rates.is_empty() {
            match self.push_rates(hotel_id, rates).await {
                Ok(()) => outcome.rates_pushed = rates.len(),
                Err(e) => {
                    tracing::error!(error = %e, "rate push failed in combined sync");
                    outcome.rates_error = Some(e.to_string());
                }
            }
        }

        outcome
    }

    // ==================== Webhook Handling ====================

    /// Process an incoming reservation notification from Booking.com.
    pub async fn process_reservation_notification(
        &self,
        xml_body: &str,
    ) -> Result<OtaHotelResNotifRS, BookingError> {
        tracing::info!("Processing reservation notification from Booking.com");

        let notification = OtaHotelResNotifRQ::from_xml(xml_body)?;

        // Process each reservation
        for notif in &notification.reservations {
            tracing::info!(
                "Processing reservation {} with status {}",
                notif.res_id,
                notif.res_status
            );
        }

        Ok(OtaHotelResNotifRS::success())
    }

    // ==================== Helper Methods ====================

    fn extract_xml_attr(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        xml.find(&pattern).and_then(|start| {
            let val_start = start + pattern.len();
            xml[val_start..]
                .find('"')
                .map(|end| xml[val_start..val_start + end].to_string())
        })
    }

    fn extract_xml_element(xml: &str, element_name: &str) -> Option<String> {
        let open_tag = format!("<{}", element_name);
        xml.find(&open_tag)
            .and_then(|tag_start| {
                xml[tag_start..].find('>').and_then(|content_rel| {
                    let content_start = tag_start + content_rel + 1;
                    let close_tag = format!("</{}>", element_name);
                    xml[content_start..]
                        .find(&close_tag)
                        .map(|end| xml[content_start..content_start + end].trim().to_string())
                })
            })
            .filter(|s| !s.is_empty())
    }
}

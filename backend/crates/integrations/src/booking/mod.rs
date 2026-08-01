//! Booking.com integration client (Story 83.2).
//!
//! Implements Booking.com Connectivity API integration,
//! including OTA XML message handling for reservations and availability.
//!
//! # Module layout
//!
//! This module is split into cohesive submodules; all public items are
//! re-exported here so existing `booking::<Item>` paths (and the crate-level
//! `pub use booking::{...}` in `lib.rs`) keep resolving unchanged:
//!
//! * [`error`]    — [`BookingError`]
//! * [`models`]   — domain types (credentials, property, reservation, mappings)
//! * [`messages`] — typed OTA request/response models
//! * [`client`]   — the [`BookingClient`] HTTP client
//! * [`ota_xml`]  — low-level quick-xml OTA (de)serialisation helpers
//! * [`oauth`]    — OAuth 2.0 client for the newer Connectivity APIs
//! * [`retry`]    — retry policy + push-outcome types
//!
//! # XML handling
//!
//! All OTA XML serialisation and deserialisation is performed via [`quick_xml`]
//! (see the [`ota_xml`] submodule).  The helpers produce and consume
//! namespace-qualified XML that complies with the OpenTravel Alliance schema
//! (`xmlns="http://www.opentravel.org/OTA/2003/05"`).  The legacy
//! string-search helpers are retained only as fallback paths for unusual
//! response shapes that the streaming parser cannot reach.

// ============================================
// OTA XML namespace
// ============================================

/// Namespace URI used by all OTA messages exchanged with Booking.com.
pub const OTA_NAMESPACE: &str = "http://www.opentravel.org/OTA/2003/05";

/// [`BookingClient`] HTTP client. See [`client`].
mod client;
/// Error type for the Booking.com integration. See [`error`].
mod error;
/// Typed OTA request/response message models. See [`messages`].
mod messages;
/// Domain model types (credentials, property, reservation, mappings). See [`models`].
mod models;

/// Low-level OTA XML serialization/deserialization helpers for the
/// Booking.com OTA wire format. See [`ota_xml`] for details.
pub mod ota_xml;

/// OAuth 2.0 (authorization-code) client for Booking.com's newer Connectivity
/// APIs. See [`oauth`] for details.
pub mod oauth;

/// Retry policy + push-outcome types for OTA rate/availability push (AC-5).
/// See [`retry`] for details.
pub mod retry;

// Re-export the submodule public items so existing `booking::<Item>` paths (and
// the crate-level `pub use booking::{...}` in `lib.rs`) keep resolving unchanged
// after the module split.
pub use client::BookingClient;
pub use error::BookingError;
pub use messages::{
    AvailStatusMessage, AvailabilityUpdate, LosRestrictions, OtaHotelAvailNotifRQ,
    OtaHotelAvailNotifRS, OtaHotelRateAmountNotifRQ, OtaHotelRateAmountNotifRS, OtaHotelResNotifRQ,
    OtaHotelResNotifRS, OtaReadRQ, OtaReadRS, OtaReservationNotification, RateUpdate,
};
pub use models::{
    map_reservation_status, BookingAddress, BookingContact, BookingCredentials, BookingGuest,
    BookingProperty, BookingReservation, BookingReservationStatus, BookingRoomType,
    PropertyMapping, RoomTypeMapping,
};
pub use oauth::{
    BookingOAuthClient, BookingOAuthConfig, BookingOAuthTokens, BOOKING_OAUTH_AUTH_URL,
    BOOKING_OAUTH_TOKEN_URL,
};
pub use retry::{BookingRetryConfig, PushOutcome};
// `is_retryable_status` is crate-private (pub(super)); the in-module tests call
// it unqualified via `use super::*`, so bring it into module scope under cfg(test).
// The `BookingClient` impl imports it directly from `retry`.
#[cfg(test)]
use retry::is_retryable_status;

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    /// A single scripted HTTP response for [`MockOtaServer`].
    struct MockResponse {
        status: u16,
        body: String,
        extra_headers: Vec<(String, String)>,
        /// When `true` the server reads the request and then drops the
        /// connection without writing any response, simulating a
        /// transport-level failure (connection reset / incomplete message)
        /// that the retry loop must treat as transient.
        drop_conn: bool,
    }

    impl MockResponse {
        fn status(status: u16, body: &str) -> Self {
            Self {
                status,
                body: body.to_string(),
                extra_headers: vec![],
                drop_conn: false,
            }
        }

        /// A response that abandons the connection mid-flight (no bytes
        /// written), forcing a `reqwest` transport error on the client.
        fn drop_connection() -> Self {
            Self {
                status: 0,
                body: String::new(),
                extra_headers: vec![],
                drop_conn: true,
            }
        }

        fn with_header(mut self, name: &str, value: &str) -> Self {
            self.extra_headers
                .push((name.to_string(), value.to_string()));
            self
        }
    }

    /// Minimal one-connection-per-request HTTP/1.1 mock server backed by a
    /// raw `tokio` TCP listener (no extra test deps). Each incoming request is
    /// answered with the next scripted [`MockResponse`]; once the script is
    /// exhausted it replies 500. Used to drive the retry/error-handling paths
    /// of the OTA push without touching the network.
    struct MockOtaServer {
        addr: std::net::SocketAddr,
        hits: Arc<AtomicUsize>,
        bodies: Arc<Mutex<Vec<String>>>,
    }

    impl MockOtaServer {
        async fn spawn(responses: Vec<MockResponse>) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let hits = Arc::new(AtomicUsize::new(0));
            let hits_task = hits.clone();
            let bodies: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let bodies_task = bodies.clone();
            let script = Arc::new(Mutex::new(responses.into_iter()));

            tokio::spawn(async move {
                loop {
                    let (mut socket, _) = match listener.accept().await {
                        Ok(pair) => pair,
                        Err(_) => break,
                    };
                    let hits_inner = hits_task.clone();
                    let bodies_inner = bodies_task.clone();
                    let script_inner = script.clone();
                    tokio::spawn(async move {
                        use tokio::io::{AsyncReadExt, AsyncWriteExt};
                        // Read the full request and capture the body so tests can
                        // assert on idempotency tokens / OTA payload shape.
                        let mut buf = [0u8; 8192];
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                        let body = raw
                            .split_once("\r\n\r\n")
                            .map(|(_, b)| b.to_string())
                            .unwrap_or_default();
                        bodies_inner.lock().unwrap().push(body);

                        hits_inner.fetch_add(1, Ordering::SeqCst);
                        let resp = {
                            let mut it = script_inner.lock().unwrap();
                            it.next()
                        };
                        // Simulate a transport failure: drop the socket without
                        // writing a response so the client observes a reset /
                        // incomplete message.
                        if resp.as_ref().is_some_and(|r| r.drop_conn) {
                            return;
                        }
                        let (status, body, extra_headers) = match resp {
                            Some(r) => (r.status, r.body, r.extra_headers),
                            None => (500, "<exhausted/>".to_string(), vec![]),
                        };
                        let reason = match status {
                            200 => "OK",
                            400 => "Bad Request",
                            429 => "Too Many Requests",
                            503 => "Service Unavailable",
                            _ => "Status",
                        };
                        let extra = extra_headers
                            .iter()
                            .map(|(k, v)| format!("{k}: {v}\r\n"))
                            .collect::<String>();
                        let response = format!(
                            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
                            body.len()
                        );
                        let _ = socket.write_all(response.as_bytes()).await;
                        let _ = socket.flush().await;
                    });
                }
            });

            Self { addr, hits, bodies }
        }

        fn url(&self) -> String {
            format!("http://{}/hotels/xml", self.addr)
        }

        fn hits(&self) -> usize {
            self.hits.load(Ordering::SeqCst)
        }

        /// Snapshot of the request bodies captured so far, in arrival order.
        fn bodies(&self) -> Vec<String> {
            self.bodies.lock().unwrap().clone()
        }
    }

    /// Extract the value of the `EchoToken` attribute from an OTA RQ root, if
    /// present. Test-only helper.
    fn echo_token_of(xml: &str) -> Option<String> {
        let marker = "EchoToken=\"";
        let start = xml.find(marker)? + marker.len();
        let rest = &xml[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }

    #[test]
    fn test_credentials_new() {
        let creds = BookingCredentials::new(
            "hotel123".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(creds.hotel_id, "hotel123");
        assert_eq!(creds.username, "user");
        assert_eq!(creds.password, "pass");
        assert!(creds.api_url.contains("booking.com"));
    }

    #[test]
    fn test_ota_read_rq_xml() {
        let rq = OtaReadRQ {
            hotel_code: "H123".to_string(),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
            status_filter: None,
        };
        let xml = rq.to_xml();
        assert!(xml.contains("OTA_ReadRQ"));
        assert!(xml.contains("H123"));
        assert!(xml.contains("2025-01-01"));
    }

    #[test]
    fn test_ota_hotel_avail_notif_rq_xml() {
        let rq = OtaHotelAvailNotifRQ {
            hotel_code: "H456".to_string(),
            avail_status_messages: vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: Some("STD".to_string()),
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 30).unwrap(),
                booking_limit: 5,
                status: "Open".to_string(),
                los_restrictions: None,
            }],
        };
        let xml = rq.to_xml();
        assert!(xml.contains("OTA_HotelAvailNotifRQ"));
        assert!(xml.contains("H456"));
        assert!(xml.contains("DBL"));
    }

    #[test]
    fn test_ota_hotel_res_notif_rs_success() {
        let rs = OtaHotelResNotifRS::success();
        let xml = rs.to_xml();
        assert!(xml.contains("OTA_HotelResNotifRS"));
        assert!(xml.contains("Success"));
    }

    #[test]
    fn test_ota_hotel_res_notif_rs_error() {
        let rs = OtaHotelResNotifRS::error("Test error");
        let xml = rs.to_xml();
        assert!(xml.contains("Errors"));
        assert!(xml.contains("Test error"));
    }

    #[test]
    fn test_reservation_status_serialization() {
        let status = BookingReservationStatus::Confirmed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"confirmed\"");

        let status = BookingReservationStatus::Cancelled;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"cancelled\"");
    }

    #[test]
    fn test_retryable_status_classification() {
        // Transient server/throttle codes are retryable.
        for code in [408, 425, 429, 500, 502, 503, 504] {
            assert!(is_retryable_status(code), "{code} should be retryable");
        }
        // Client errors (bad message / auth) are NOT retryable.
        for code in [400, 401, 403, 404, 409, 422] {
            assert!(!is_retryable_status(code), "{code} should not be retryable");
        }
    }

    #[test]
    fn test_retry_backoff_is_exponential_and_capped() {
        let cfg = BookingRetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 1_000,
            backoff_multiplier: 2,
        };
        assert_eq!(cfg.delay_for_attempt(0), 100); // 100 * 2^0
        assert_eq!(cfg.delay_for_attempt(1), 200); // 100 * 2^1
        assert_eq!(cfg.delay_for_attempt(2), 400); // 100 * 2^2
        assert_eq!(cfg.delay_for_attempt(3), 800); // 100 * 2^3
        assert_eq!(cfg.delay_for_attempt(4), 1_000); // 1600 capped to 1000
                                                     // Large attempt counts must not overflow.
        assert_eq!(cfg.delay_for_attempt(64), 1_000);
    }

    #[test]
    fn test_retry_config_defaults_and_no_retry() {
        let def = BookingRetryConfig::default();
        assert!(def.max_attempts >= 1);
        let none = BookingRetryConfig::no_retry();
        assert_eq!(none.max_attempts, 1);
        assert_eq!(none.delay_for_attempt(0), 0);
    }

    #[test]
    fn test_effective_delay_uses_backoff_when_no_retry_after() {
        // Without a Retry-After hint, effective_delay_ms is exactly the
        // exponential backoff for the given attempt.
        let cfg = BookingRetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2,
        };
        assert_eq!(cfg.effective_delay_ms(0, None), 100);
        assert_eq!(cfg.effective_delay_ms(1, None), 200);
        assert_eq!(cfg.effective_delay_ms(2, None), 400);
        // Matches delay_for_attempt for every attempt when no hint is present.
        for attempt in 0..6 {
            assert_eq!(
                cfg.effective_delay_ms(attempt, None),
                cfg.delay_for_attempt(attempt)
            );
        }
    }

    #[test]
    fn test_effective_delay_takes_max_of_hint_and_backoff() {
        let cfg = BookingRetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2,
        };
        // Hint larger than backoff -> hint wins (never retry sooner than the
        // server asked).
        assert_eq!(cfg.effective_delay_ms(0, Some(5_000)), 5_000);
        // Backoff larger than hint -> backoff wins (never retry sooner than our
        // own schedule).
        assert_eq!(cfg.effective_delay_ms(3, Some(100)), 800);
        // Equal -> that value.
        assert_eq!(cfg.effective_delay_ms(1, Some(200)), 200);
        // A zero hint never shortens the backoff below its scheduled value.
        assert_eq!(cfg.effective_delay_ms(2, Some(0)), 400);
    }

    #[tokio::test]
    async fn test_push_availability_retries_then_succeeds_on_transient_5xx() {
        // First call returns 503 (retryable), second returns an OTA success
        // body. The push must retry and ultimately succeed.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(
                200,
                "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 3,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 4,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert!(result.is_ok(), "expected success after retry: {result:?}");
        assert_eq!(server.hits(), 2, "expected exactly one retry");
    }

    #[tokio::test]
    async fn test_push_rates_does_not_retry_on_client_error() {
        // 400 is non-retryable: the push must fail after a single attempt.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(400, "<bad-request/>"),
            // Second response should never be consumed.
            MockResponse::status(200, "<Success/>"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 4,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![RateUpdate {
            room_type_id: "DBL".to_string(),
            rate_plan_code: "STD".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            base_rate: "120.00".parse().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }];

        let result = client.push_rates("H1", &updates).await;
        assert!(result.is_err(), "400 must not be retried into success");
        assert_eq!(server.hits(), 1, "client error must not be retried");
    }

    #[tokio::test]
    async fn test_push_rates_exhausts_retries_on_persistent_5xx() {
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(503, "<busy/>"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![RateUpdate {
            room_type_id: "DBL".to_string(),
            rate_plan_code: "STD".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            base_rate: "120.00".parse().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }];

        let result = client.push_rates("H1", &updates).await;
        assert!(result.is_err(), "persistent 5xx must fail");
        assert_eq!(server.hits(), 2, "should exhaust exactly max_attempts");
    }

    #[tokio::test]
    async fn test_push_availability_retries_after_transport_error() {
        // First attempt: the server drops the connection without replying,
        // producing a reqwest transport error. That must be treated as
        // transient and retried; the second attempt returns an OTA success.
        let server = MockOtaServer::spawn(vec![
            MockResponse::drop_connection(),
            MockResponse::status(
                200,
                "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(3));

        let result = client.push_availability("H1", &[avail_update(4)]).await;
        assert!(
            result.is_ok(),
            "transport error on first attempt must be retried into success: {result:?}"
        );
        assert_eq!(
            server.hits(),
            2,
            "expected exactly one retry after the reset"
        );
    }

    #[tokio::test]
    async fn test_push_rates_exhausts_retries_on_persistent_transport_error() {
        // Every attempt drops the connection: the push must exhaust its
        // attempts and surface a Network error rather than looping forever.
        let server = MockOtaServer::spawn(vec![
            MockResponse::drop_connection(),
            MockResponse::drop_connection(),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(2));

        let result = client.push_rates("H1", &[rate_update("120.00")]).await;
        assert!(
            matches!(result, Err(BookingError::Network(_))),
            "persistent transport failure must surface as Network error: {result:?}"
        );
        assert_eq!(server.hits(), 2, "should exhaust exactly max_attempts");
    }

    #[tokio::test]
    async fn test_push_429_with_retry_after_returns_rate_limited() {
        // Both attempts return 429 with Retry-After: 0 so the test stays fast
        // (delay capped to 0 via max_delay_ms=0). Verify:
        //   a) all attempts are consumed, and
        //   b) the final error is RateLimited, not a generic Api error.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(429, "<rate-limited/>").with_header("Retry-After", "0"),
            MockResponse::status(429, "<rate-limited/>").with_header("Retry-After", "0"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 2,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert!(
            matches!(result, Err(BookingError::RateLimited(_))),
            "429 with Retry-After must surface as RateLimited, got: {result:?}"
        );
        assert_eq!(server.hits(), 2, "should exhaust exactly max_attempts");
    }

    #[tokio::test]
    async fn test_push_429_without_retry_after_returns_rate_limited_zero() {
        // Naked 429 with no Retry-After header must also surface as
        // RateLimited(0), not Api("HTTP 429: …").
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(429, "<too-many/>"),
            MockResponse::status(429, "<too-many/>"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 2,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert_eq!(
            server.hits(),
            2,
            "should exhaust max_attempts on persistent 429"
        );
        match result {
            Err(BookingError::RateLimited(secs)) => {
                assert_eq!(secs, 0, "no Retry-After header → RateLimited(0)");
            }
            other => panic!("expected RateLimited(0), got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_push_429_retry_after_secs_propagated_to_error() {
        // Retry-After: 5 must propagate into RateLimited(5).
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(429, "<too-many/>").with_header("Retry-After", "5"),
            MockResponse::status(429, "<too-many/>").with_header("Retry-After", "5"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        // max_delay_ms = 0 clamps the Retry-After sleep to 0 so the test
        // stays fast; the important assertion is the error value, not the timing.
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 2,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert_eq!(server.hits(), 2, "should exhaust max_attempts");
        match result {
            Err(BookingError::RateLimited(secs)) => {
                assert_eq!(secs, 5, "Retry-After: 5 → RateLimited(5)");
            }
            other => panic!("expected RateLimited(5), got: {other:?}"),
        }
    }

    // ==================== Idempotency (AC-5) ====================

    fn avail_update(count: i32) -> AvailabilityUpdate {
        AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: count,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }
    }

    fn rate_update(rate: &str) -> RateUpdate {
        RateUpdate {
            room_type_id: "DBL".to_string(),
            rate_plan_code: "STD".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            base_rate: rate.parse().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }
    }

    #[test]
    fn test_echo_token_is_deterministic_for_identical_payload() {
        let a = ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01");
        let b = ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01");
        assert_eq!(
            a, b,
            "identical payload must yield identical idempotency key"
        );
        assert_eq!(a.len(), 32, "token is a 128-bit hex digest");
    }

    #[test]
    fn test_echo_token_differs_for_different_payload_and_kind() {
        let base = ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01:4");
        // Different content -> different token.
        assert_ne!(
            base,
            ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01:5")
        );
        // Same content, different message kind -> different token (no collision
        // between an availability push and a rate push).
        assert_ne!(
            base,
            ota_xml::compute_echo_token("rate", "H1|DBL:STD:2025-06-01:4")
        );
    }

    #[test]
    fn test_avail_xml_stamps_deterministic_echo_token() {
        let rq = OtaHotelAvailNotifRQ {
            hotel_code: "H1".to_string(),
            avail_status_messages: vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: Some("STD".to_string()),
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                booking_limit: 4,
                status: "Open".to_string(),
                los_restrictions: None,
            }],
        };
        let first = rq.to_xml();
        let second = rq.to_xml();
        let t1 = echo_token_of(&first).expect("avail RQ must carry an EchoToken");
        let t2 = echo_token_of(&second).expect("avail RQ must carry an EchoToken");
        assert_eq!(t1, t2, "rebuilding the same push must reuse the same token");
    }

    #[test]
    fn test_rate_xml_stamps_echo_token_and_changes_with_content() {
        let token_for = |rate: &str| {
            let xml = ota_xml::build_rate_amount_notif_rq(
                "H1",
                &[(
                    NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                    "DBL",
                    "STD",
                    &rate.parse::<Decimal>().unwrap(),
                    "EUR",
                )],
            )
            .unwrap();
            echo_token_of(&xml).expect("rate RQ must carry an EchoToken")
        };
        assert_eq!(token_for("120.00"), token_for("120.00"));
        assert_ne!(token_for("120.00"), token_for("130.00"));
    }

    #[tokio::test]
    async fn test_push_availability_reuses_echo_token_across_retries() {
        // 503 then 200: the push retries. Both requests must carry the SAME
        // EchoToken so the upstream can deduplicate the retried delivery.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(
                200,
                "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(3));

        let result = client.push_availability("H1", &[avail_update(4)]).await;
        assert!(result.is_ok(), "expected success after retry: {result:?}");
        assert_eq!(server.hits(), 2, "expected exactly one retry");

        let bodies = server.bodies();
        assert_eq!(bodies.len(), 2, "both delivery attempts captured");
        let t0 = echo_token_of(&bodies[0]).expect("attempt 0 must carry a token");
        let t1 = echo_token_of(&bodies[1]).expect("attempt 1 must carry a token");
        assert_eq!(
            t0, t1,
            "retried delivery must reuse the same idempotency key"
        );
    }

    #[tokio::test]
    async fn test_push_rates_reuses_echo_token_across_retries() {
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(
                200,
                "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelRateAmountNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(3));

        let result = client.push_rates("H1", &[rate_update("120.00")]).await;
        assert!(result.is_ok(), "expected success after retry: {result:?}");
        assert_eq!(server.hits(), 2);

        let bodies = server.bodies();
        let t0 = echo_token_of(&bodies[0]).expect("attempt 0 token");
        let t1 = echo_token_of(&bodies[1]).expect("attempt 1 token");
        assert_eq!(t0, t1, "rate retry must reuse the same idempotency key");
    }

    // ==================== Combined push (PushOutcome, AC-5) ====================

    const AVAIL_OK_BODY: &str = "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>";
    const RATE_OK_BODY: &str = "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelRateAmountNotifRS>";
    const ERR_BODY: &str = "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Errors><Error Type=\"3\" Code=\"450\" ShortText=\"Rate rejected\"/></Errors></OTA_HotelRateAmountNotifRS>";

    fn client_for(server: &MockOtaServer) -> BookingClient {
        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        BookingClient::new(creds).with_retry(BookingRetryConfig::no_retry())
    }

    #[test]
    fn test_push_outcome_success_and_partial_helpers() {
        let clean = PushOutcome {
            availability_pushed: 2,
            rates_pushed: 1,
            availability_error: None,
            rates_error: None,
        };
        assert!(clean.is_success());
        assert!(!clean.is_partial());

        let half = PushOutcome {
            availability_pushed: 2,
            rates_pushed: 0,
            availability_error: None,
            rates_error: Some("Rate rejected".to_string()),
        };
        assert!(!half.is_success());
        assert!(half.is_partial());

        let both = PushOutcome {
            availability_pushed: 0,
            rates_pushed: 0,
            availability_error: Some("a".to_string()),
            rates_error: Some("b".to_string()),
        };
        assert!(!both.is_success());
        // Both failed -> not partial (it's a total failure, nothing landed).
        assert!(!both.is_partial());
    }

    #[tokio::test]
    async fn test_combined_push_both_streams_succeed() {
        // Availability push (1 HTTP call) then rate push (1 HTTP call).
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(200, AVAIL_OK_BODY),
            MockResponse::status(200, RATE_OK_BODY),
        ])
        .await;
        let client = client_for(&server);

        let outcome = client
            .push_availability_and_rates("H1", &[avail_update(4)], &[rate_update("120.00")])
            .await;

        assert!(
            outcome.is_success(),
            "both streams should apply: {outcome:?}"
        );
        assert!(!outcome.is_partial());
        assert_eq!(outcome.availability_pushed, 1);
        assert_eq!(outcome.rates_pushed, 1);
        assert_eq!(server.hits(), 2, "one HTTP call per stream");
    }

    #[tokio::test]
    async fn test_combined_push_availability_ok_rates_fail_is_partial() {
        // Availability succeeds; rates come back with an OTA <Errors> body.
        // The availability success must NOT be lost just because rates failed.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(200, AVAIL_OK_BODY),
            MockResponse::status(200, ERR_BODY),
        ])
        .await;
        let client = client_for(&server);

        let outcome = client
            .push_availability_and_rates("H1", &[avail_update(4)], &[rate_update("120.00")])
            .await;

        assert!(!outcome.is_success());
        assert!(
            outcome.is_partial(),
            "exactly one stream failed: {outcome:?}"
        );
        assert_eq!(outcome.availability_pushed, 1, "availability still applied");
        assert_eq!(outcome.rates_pushed, 0, "rates did not apply");
        assert!(outcome.availability_error.is_none());
        assert!(
            outcome
                .rates_error
                .as_deref()
                .is_some_and(|e| e.contains("Rate rejected")),
            "rate error text must be surfaced: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn test_combined_push_no_op_when_both_empty() {
        // No availability and no rates: a successful no-op that makes zero
        // HTTP calls.
        let server = MockOtaServer::spawn(vec![]).await;
        let client = client_for(&server);

        let outcome = client.push_availability_and_rates("H1", &[], &[]).await;

        assert!(outcome.is_success());
        assert!(!outcome.is_partial());
        assert_eq!(outcome.availability_pushed, 0);
        assert_eq!(outcome.rates_pushed, 0);
        assert_eq!(server.hits(), 0, "empty sync must not hit the network");
    }

    #[tokio::test]
    async fn test_combined_push_continues_rates_after_availability_failure() {
        // Availability fails (non-retryable 400) but rates must still be
        // attempted — the failure of one stream cannot abort the other.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(400, "<bad-request/>"),
            MockResponse::status(200, RATE_OK_BODY),
        ])
        .await;
        let client = client_for(&server);

        let outcome = client
            .push_availability_and_rates("H1", &[avail_update(4)], &[rate_update("99.00")])
            .await;

        assert!(outcome.is_partial(), "one stream failed: {outcome:?}");
        assert_eq!(outcome.availability_pushed, 0);
        assert_eq!(
            outcome.rates_pushed, 1,
            "rates attempted despite avail failure"
        );
        assert!(outcome.availability_error.is_some());
        assert!(outcome.rates_error.is_none());
        assert_eq!(server.hits(), 2, "both streams hit the network");
    }

    #[test]
    fn test_map_reservation_status() {
        // Commit -> Confirmed
        let xml = r#"<HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-001"/></HotelReservationIDs></ResGlobalInfo></HotelReservation>"#;
        let result = OtaReadRS::parse_single_reservation(xml);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.status, BookingReservationStatus::Confirmed);

        // Cancel -> Cancelled
        let xml = r#"<HotelReservation ResStatus="Cancel"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-002"/></HotelReservationIDs></ResGlobalInfo></HotelReservation>"#;
        let result = OtaReadRS::parse_single_reservation(xml);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.status, BookingReservationStatus::Cancelled);
    }

    // ----------------------------------------------------------------------
    // OtaReadRS::from_xml — response-envelope classification.
    //
    // parse_single_reservation is covered above; these lock in the four
    // envelope branches of from_xml (explicit <Success/>, implicit
    // <ReservationsList>, an <Errors> response, and the indeterminate
    // no-marker case) plus the multi-reservation fan-out. Behaviour-only
    // characterisation — no production code changes.
    // ----------------------------------------------------------------------

    #[test]
    fn test_from_xml_explicit_success_parses_reservation() {
        // <Success/> present, no <ReservationsList> wrapper: the explicit
        // success branch must still parse the embedded reservation.
        let xml = r#"<OTA_ReadRS><Success/><HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-EXPL-1"/></HotelReservationIDs></ResGlobalInfo></HotelReservation></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(rs.success, "explicit <Success/> is a success envelope");
        assert!(rs.error.is_none());
        assert_eq!(rs.reservations.len(), 1);
        assert_eq!(rs.reservations[0].reservation_id, "BK-EXPL-1");
        assert_eq!(
            rs.reservations[0].status,
            BookingReservationStatus::Confirmed
        );
    }

    #[test]
    fn test_from_xml_implicit_success_via_reservations_list() {
        // No <Success/>, but a <ReservationsList> element implies success
        // for the Booking.com variants that omit the explicit marker.
        let xml = r#"<OTA_ReadRS><ReservationsList><HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-IMPL-1"/></HotelReservationIDs></ResGlobalInfo></HotelReservation></ReservationsList></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(rs.success, "<ReservationsList> is an implicit success");
        assert!(rs.error.is_none());
        assert_eq!(rs.reservations.len(), 1);
        assert_eq!(rs.reservations[0].reservation_id, "BK-IMPL-1");
    }

    #[test]
    fn test_from_xml_error_response_is_unsuccessful() {
        // An <Errors> envelope must surface success=false with the
        // ShortText carried through as the error message, and no
        // reservations parsed.
        let xml = r#"<OTA_ReadRS><Errors><Error Type="1" ShortText="Authentication failed"/></Errors></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(!rs.success, "<Errors> envelope is not a success");
        assert_eq!(rs.error.as_deref(), Some("Authentication failed"));
        assert!(rs.reservations.is_empty());
    }

    #[test]
    fn test_from_xml_indeterminate_response_is_unsuccessful() {
        // No <Success>, no <Errors>, no <ReservationsList>: the parser must
        // NOT silently treat this as success (that would mask a failed
        // pull). success=false with a non-empty error and no reservations.
        let xml = r#"<OTA_ReadRS></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(!rs.success, "indeterminate envelope must not be success");
        assert!(
            rs.error.is_some(),
            "indeterminate response must carry an error"
        );
        assert!(rs.reservations.is_empty());
    }

    #[test]
    fn test_from_xml_parses_multiple_reservations() {
        // The parse_reservations fan-out must return one entry per
        // <HotelReservation> element, in document order.
        let xml = r#"<OTA_ReadRS><Success/><ReservationsList><HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-A"/></HotelReservationIDs></ResGlobalInfo></HotelReservation><HotelReservation ResStatus="Cancel"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-B"/></HotelReservationIDs></ResGlobalInfo></HotelReservation></ReservationsList></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(rs.success);
        assert_eq!(rs.reservations.len(), 2);
        assert_eq!(rs.reservations[0].reservation_id, "BK-A");
        assert_eq!(
            rs.reservations[0].status,
            BookingReservationStatus::Confirmed
        );
        assert_eq!(rs.reservations[1].reservation_id, "BK-B");
        assert_eq!(
            rs.reservations[1].status,
            BookingReservationStatus::Cancelled
        );
    }

    // ----------------------------------------------------------------------
    // OtaHotelRateAmountNotifRQ / RS typed request/response models (Story 83.2)
    // ----------------------------------------------------------------------

    fn rate_update_on(room: &str, rate: &str, date: NaiveDate) -> RateUpdate {
        RateUpdate {
            room_type_id: room.to_string(),
            rate_plan_code: "STD".to_string(),
            date,
            base_rate: rate.parse::<Decimal>().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }
    }

    #[test]
    fn test_rate_amount_notif_rq_to_xml_shape() {
        let rq = OtaHotelRateAmountNotifRQ {
            hotel_code: "H-RT-99".to_string(),
            rate_amount_messages: vec![rate_update_on(
                "DBL",
                "120.50",
                NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
            )],
        };
        let xml = rq.to_xml().expect("serialisation must succeed");
        assert!(xml.contains("OTA_HotelRateAmountNotifRQ"));
        assert!(xml.contains(&format!("xmlns=\"{OTA_NAMESPACE}\"")));
        assert!(xml.contains("HotelCode=\"H-RT-99\""));
        assert!(xml.contains("InvTypeCode=\"DBL\""));
        assert!(xml.contains("RatePlanCode=\"STD\""));
        assert!(xml.contains("AmountAfterTax=\"120.50\""));
        assert!(xml.contains("CurrencyCode=\"EUR\""));
        // Single-day update: Start == End.
        assert!(xml.contains("Start=\"2025-07-01\""));
        assert!(xml.contains("End=\"2025-07-01\""));
    }

    #[test]
    fn test_rate_amount_notif_rq_from_xml_parses_typed_model() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <RateAmountMessages HotelCode="H-IN-7">
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-08-10" End="2025-08-10" InvTypeCode="STE" RatePlanCode="FLEX"/>
      <Rates><Rate><BaseByGuestAmts>
        <BaseByGuestAmt AmountAfterTax="250.00" CurrencyCode="EUR"/>
      </BaseByGuestAmts></Rate></Rates>
    </RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        let rq = OtaHotelRateAmountNotifRQ::from_xml(xml).expect("parse must succeed");
        assert_eq!(rq.hotel_code, "H-IN-7");
        assert_eq!(rq.rate_amount_messages.len(), 1);
        let m = &rq.rate_amount_messages[0];
        assert_eq!(m.room_type_id, "STE");
        assert_eq!(m.rate_plan_code, "FLEX");
        assert_eq!(m.date, NaiveDate::from_ymd_opt(2025, 8, 10).unwrap());
        assert_eq!(m.base_rate, "250.00".parse::<Decimal>().unwrap());
        assert_eq!(m.currency, "EUR");
        assert_eq!(m.extra_person_rate, None);
    }

    #[test]
    fn test_rate_amount_notif_rq_round_trip() {
        let original = OtaHotelRateAmountNotifRQ {
            hotel_code: "H-RT-RT".to_string(),
            rate_amount_messages: vec![
                rate_update_on("DBL", "99.00", NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()),
                rate_update_on(
                    "STE",
                    "180.00",
                    NaiveDate::from_ymd_opt(2025, 9, 2).unwrap(),
                ),
            ],
        };
        let xml = original.to_xml().unwrap();
        let parsed = OtaHotelRateAmountNotifRQ::from_xml(&xml).unwrap();
        assert_eq!(parsed.hotel_code, original.hotel_code);
        assert_eq!(parsed.rate_amount_messages.len(), 2);
        for (got, want) in parsed
            .rate_amount_messages
            .iter()
            .zip(original.rate_amount_messages.iter())
        {
            assert_eq!(got.room_type_id, want.room_type_id);
            assert_eq!(got.rate_plan_code, want.rate_plan_code);
            assert_eq!(got.date, want.date);
            assert_eq!(got.base_rate, want.base_rate);
            assert_eq!(got.currency, want.currency);
        }
    }

    #[test]
    fn test_rate_amount_notif_rq_from_xml_bad_amount_errors() {
        let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H">
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-08-10" End="2025-08-10" InvTypeCode="STE" RatePlanCode="FLEX"/>
      <Rates><Rate><BaseByGuestAmts>
        <BaseByGuestAmt AmountAfterTax="not-a-number" CurrencyCode="EUR"/>
      </BaseByGuestAmts></Rate></Rates>
    </RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        assert!(OtaHotelRateAmountNotifRQ::from_xml(xml).is_err());
    }

    #[test]
    fn test_rate_amount_notif_rs_success_and_error() {
        let ok = OtaHotelRateAmountNotifRS::from_xml(
            "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelRateAmountNotifRS>",
        )
        .unwrap();
        assert!(ok.success);
        assert!(ok.error.is_none());

        let err = OtaHotelRateAmountNotifRS::from_xml(
            "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Errors><Error ShortText=\"rate rejected\"/></Errors></OTA_HotelRateAmountNotifRS>",
        )
        .unwrap();
        assert!(!err.success);
        assert_eq!(err.error.as_deref(), Some("rate rejected"));
    }

    // ==================== fetch_property outbound happy path ====================

    /// A well-formed `OTA_HotelDescriptiveInfoRS` success body carrying the
    /// property fields `parse_property_response` extracts. Deliberately free of
    /// any `<Errors>` / `<Error` marker so the parser takes the success branch.
    const HOTEL_DESCRIPTIVE_INFO_RS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelDescriptiveInfoRS xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <Success/>
  <HotelDescriptiveContents>
    <HotelDescriptiveContent HotelCode="H42" HotelName="Grand Test Hotel">
      <HotelInfo Rating="4">
        <Descriptions>
          <DescriptiveText>A lovely seaside hotel.</DescriptiveText>
        </Descriptions>
        <Position Latitude="48.1486" Longitude="17.1077"/>
      </HotelInfo>
      <ContactInfos>
        <ContactInfo>
          <Address CountryCode="SK">
            <AddressLine>123 Ocean Drive</AddressLine>
            <CityName>Bratislava</CityName>
            <StateProv>BL</StateProv>
            <PostalCode>81101</PostalCode>
          </Address>
          <Email>info@grandtest.example</Email>
          <Phone PhoneNumber="+421900000000"/>
          <URL>https://grandtest.example</URL>
        </ContactInfo>
      </ContactInfos>
      <Policies>
        <Policy>
          <PolicyInfo CheckInTime="14:00" CheckOutTime="11:00"/>
        </Policy>
      </Policies>
    </HotelDescriptiveContent>
  </HotelDescriptiveContents>
</OTA_HotelDescriptiveInfoRS>"#;

    /// The connect / sync "happy path" hinges on `fetch_property` issuing a real
    /// `OTA_HotelDescriptiveInfoRQ` POST and parsing the descriptive-info
    /// response — a path that was previously only reachable against the live
    /// Booking.com endpoint (documented limitation). The base-URL seam
    /// (`BookingCredentials::with_url`, mirroring Airbnb's `with_base_url`
    /// from #2240) lets us point the client at [`MockOtaServer`] and exercise
    /// the outbound call + response parsing deterministically.
    #[tokio::test]
    async fn test_fetch_property_happy_path_issues_request_and_parses_response() {
        let server =
            MockOtaServer::spawn(vec![MockResponse::status(200, HOTEL_DESCRIPTIVE_INFO_RS)]).await;

        let creds = BookingCredentials::with_url(
            "H42".to_string(),
            "user".to_string(),
            "pass".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds);

        let property = client
            .fetch_property("H42")
            .await
            .expect("fetch_property should succeed on a well-formed OTA response");

        // Exactly one outbound call was made.
        assert_eq!(server.hits(), 1, "fetch_property should POST once");

        // The outbound request is a namespaced OTA_HotelDescriptiveInfoRQ that
        // carries the requested hotel code.
        let sent = server.bodies();
        assert_eq!(sent.len(), 1);
        assert!(
            sent[0].contains("OTA_HotelDescriptiveInfoRQ"),
            "outbound body should be an OTA_HotelDescriptiveInfoRQ, got: {}",
            sent[0]
        );
        assert!(
            sent[0].contains("HotelCode=\"H42\""),
            "outbound body should target the requested hotel code, got: {}",
            sent[0]
        );

        // The response was parsed into the typed property.
        assert_eq!(property.hotel_id, "H42");
        assert_eq!(property.name, "Grand Test Hotel");
        assert_eq!(
            property.description.as_deref(),
            Some("A lovely seaside hotel.")
        );
        assert_eq!(property.star_rating, Some(4));
        assert_eq!(property.address.street, "123 Ocean Drive");
        assert_eq!(property.address.city, "Bratislava");
        assert_eq!(property.address.state.as_deref(), Some("BL"));
        assert_eq!(property.address.postal_code, "81101");
        assert_eq!(property.address.country_code, "SK");
        assert_eq!(
            property.contact.email.as_deref(),
            Some("info@grandtest.example")
        );
        assert_eq!(property.contact.phone.as_deref(), Some("+421900000000"));
        assert_eq!(
            property.contact.website.as_deref(),
            Some("https://grandtest.example")
        );
        assert_eq!(property.check_in_time.as_deref(), Some("14:00"));
        assert_eq!(property.check_out_time.as_deref(), Some("11:00"));
        assert!(property.synced_at.is_some());
    }

    /// `fetch_properties` fans out to `fetch_property` for the configured hotel
    /// id; on the happy path it returns the single parsed property (rather than
    /// the placeholder fallback used when the descriptive-info call fails).
    #[tokio::test]
    async fn test_fetch_properties_happy_path_returns_parsed_property() {
        let server =
            MockOtaServer::spawn(vec![MockResponse::status(200, HOTEL_DESCRIPTIVE_INFO_RS)]).await;

        let creds = BookingCredentials::with_url(
            "H42".to_string(),
            "user".to_string(),
            "pass".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds);

        let properties = client
            .fetch_properties()
            .await
            .expect("fetch_properties should succeed");

        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0].hotel_id, "H42");
        // The real parsed name proves we took the success path, not the
        // "Property {hotel_id}" placeholder built on fetch failure.
        assert_eq!(properties[0].name, "Grand Test Hotel");
    }
}

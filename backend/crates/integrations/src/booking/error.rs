//! Error type for the Booking.com integration.

use thiserror::Error;

/// Errors that can occur during Booking.com API operations.
#[derive(Debug, Error)]
pub enum BookingError {
    /// OAuth/authentication error.
    #[error("Authentication error: {0}")]
    Auth(String),

    /// API error from Booking.com.
    #[error("API error: {0}")]
    Api(String),

    /// Network/HTTP error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// XML parsing error.
    #[error("XML error: {0}")]
    Xml(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimited(u64),

    /// Push operation failed.
    #[error("Push failed: {0}")]
    PushFailed(String),

    /// Invalid OTA message.
    #[error("Invalid OTA message: {0}")]
    InvalidMessage(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}

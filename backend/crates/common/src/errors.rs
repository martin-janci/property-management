//! Common error types.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Standard API error response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Request ID for support/debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Detailed validation errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<ValidationError>>,
    /// ISO 8601 timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            request_id: None,
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_details(mut self, details: Vec<ValidationError>) -> Self {
        self.details = Some(details);
        self
    }

    /// Create a bad request (400) error response.
    pub fn bad_request(message: &str) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    /// Create a forbidden (403) error response.
    pub fn forbidden(message: &str) -> Self {
        Self::new("FORBIDDEN", message)
    }

    /// Create a not found (404) error response.
    pub fn not_found(message: &str) -> Self {
        Self::new("NOT_FOUND", message)
    }

    /// Create an internal server error (500) response.
    pub fn internal_error(message: &str) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }
}

/// Validation error detail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationError {
    /// Field path (e.g., "address.city")
    pub field: String,
    /// Error message for this field
    pub message: String,
    /// Error code
    pub code: String,
}

/// Application error types.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("External service error: {0}")]
    ExternalService(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            AppError::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::ExternalService(_) => "EXTERNAL_SERVICE_ERROR",
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            AppError::BadRequest(_) => 400,
            AppError::Unauthorized(_) => 401,
            AppError::Forbidden(_) => 403,
            AppError::NotFound(_) => 404,
            AppError::Conflict(_) => 409,
            AppError::UnprocessableEntity(_) => 422,
            AppError::RateLimitExceeded => 429,
            AppError::Internal(_) | AppError::Database(_) | AppError::ExternalService(_) => 500,
        }
    }

    pub fn to_response(&self) -> ErrorResponse {
        ErrorResponse::new(self.code(), self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ErrorResponse builders ----

    #[test]
    fn new_populates_required_fields_and_skips_optional() {
        let resp = ErrorResponse::new("BAD_REQUEST", "missing email");
        assert_eq!(resp.code, "BAD_REQUEST");
        assert_eq!(resp.message, "missing email");
        assert!(resp.request_id.is_none());
        assert!(resp.details.is_none());
    }

    #[test]
    fn timestamp_is_close_to_now() {
        let before = chrono::Utc::now();
        let resp = ErrorResponse::new("X", "y");
        let after = chrono::Utc::now();
        assert!(resp.timestamp >= before);
        assert!(resp.timestamp <= after);
    }

    #[test]
    fn with_request_id_sets_only_request_id() {
        let resp = ErrorResponse::new("X", "y").with_request_id("req-123");
        assert_eq!(resp.request_id.as_deref(), Some("req-123"));
        assert!(resp.details.is_none());
    }

    #[test]
    fn with_details_attaches_validation_errors() {
        let details = vec![ValidationError {
            field: "email".to_string(),
            message: "invalid format".to_string(),
            code: "INVALID_EMAIL".to_string(),
        }];
        let resp = ErrorResponse::new("VALIDATION_ERROR", "validation failed")
            .with_details(details.clone());

        let actual = resp.details.expect("details should be set");
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].field, "email");
        assert_eq!(actual[0].code, "INVALID_EMAIL");
    }

    #[test]
    fn convenience_builders_use_documented_codes() {
        assert_eq!(ErrorResponse::bad_request("x").code, "BAD_REQUEST");
        assert_eq!(ErrorResponse::forbidden("x").code, "FORBIDDEN");
        assert_eq!(ErrorResponse::not_found("x").code, "NOT_FOUND");
        assert_eq!(ErrorResponse::internal_error("x").code, "INTERNAL_ERROR");
    }

    // ---- Serde contract ----

    #[test]
    fn error_response_skips_optional_fields_in_json() {
        let resp = ErrorResponse::new("BAD_REQUEST", "missing email");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"code\":\"BAD_REQUEST\""));
        assert!(json.contains("\"message\":\"missing email\""));
        assert!(!json.contains("request_id"));
        assert!(!json.contains("details"));
        assert!(json.contains("timestamp"));
    }

    #[test]
    fn error_response_serializes_details_when_present() {
        let resp = ErrorResponse::new("VALIDATION_ERROR", "bad input").with_details(vec![
            ValidationError {
                field: "email".to_string(),
                message: "invalid".to_string(),
                code: "INVALID".to_string(),
            },
        ]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"details\""));
        assert!(json.contains("\"field\":\"email\""));
    }

    // ---- AppError code mapping ----

    #[test]
    fn app_error_code_matches_variant() {
        assert_eq!(AppError::BadRequest("x".into()).code(), "BAD_REQUEST");
        assert_eq!(AppError::Unauthorized("x".into()).code(), "UNAUTHORIZED");
        assert_eq!(AppError::Forbidden("x".into()).code(), "FORBIDDEN");
        assert_eq!(AppError::NotFound("x".into()).code(), "NOT_FOUND");
        assert_eq!(AppError::Conflict("x".into()).code(), "CONFLICT");
        assert_eq!(
            AppError::UnprocessableEntity("x".into()).code(),
            "UNPROCESSABLE_ENTITY"
        );
        assert_eq!(AppError::RateLimitExceeded.code(), "RATE_LIMIT_EXCEEDED");
        assert_eq!(AppError::Internal("x".into()).code(), "INTERNAL_ERROR");
        assert_eq!(AppError::Database("x".into()).code(), "DATABASE_ERROR");
        assert_eq!(
            AppError::ExternalService("x".into()).code(),
            "EXTERNAL_SERVICE_ERROR"
        );
    }

    // ---- AppError status code mapping ----

    #[test]
    fn app_error_status_code_matches_http_semantics() {
        assert_eq!(AppError::BadRequest("x".into()).status_code(), 400);
        assert_eq!(AppError::Unauthorized("x".into()).status_code(), 401);
        assert_eq!(AppError::Forbidden("x".into()).status_code(), 403);
        assert_eq!(AppError::NotFound("x".into()).status_code(), 404);
        assert_eq!(AppError::Conflict("x".into()).status_code(), 409);
        assert_eq!(AppError::UnprocessableEntity("x".into()).status_code(), 422);
        assert_eq!(AppError::RateLimitExceeded.status_code(), 429);
        assert_eq!(AppError::Internal("x".into()).status_code(), 500);
        assert_eq!(AppError::Database("x".into()).status_code(), 500);
        assert_eq!(AppError::ExternalService("x".into()).status_code(), 500);
    }

    // ---- AppError -> ErrorResponse round-trip ----

    #[test]
    fn to_response_propagates_code_and_message() {
        let err = AppError::NotFound("user not found".into());
        let resp = err.to_response();

        assert_eq!(resp.code, "NOT_FOUND");
        // Display uses the `#[error(...)]` format string.
        assert_eq!(resp.message, "Not found: user not found");
        assert!(resp.details.is_none());
    }

    #[test]
    fn to_response_handles_unit_variant() {
        let resp = AppError::RateLimitExceeded.to_response();
        assert_eq!(resp.code, "RATE_LIMIT_EXCEEDED");
        assert_eq!(resp.message, "Rate limit exceeded");
    }

    // ---- Display impl from thiserror ----

    #[test]
    fn display_uses_thiserror_format_strings() {
        assert_eq!(
            AppError::BadRequest("missing field".into()).to_string(),
            "Bad request: missing field"
        );
        assert_eq!(
            AppError::Forbidden("admin only".into()).to_string(),
            "Forbidden: admin only"
        );
        assert_eq!(
            AppError::Internal("oops".into()).to_string(),
            "Internal server error: oops"
        );
        assert_eq!(
            AppError::RateLimitExceeded.to_string(),
            "Rate limit exceeded"
        );
    }
}

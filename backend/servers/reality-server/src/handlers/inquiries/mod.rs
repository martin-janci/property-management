//! Inquiry handlers - contact forms and viewing requests.
//!
//! Implements contact validation, email notifications,
//! and viewing scheduling functionality.

use api_core::middleware::RateLimitDecision;
use db::models::{CreateListingInquiry, ListingInquiry, ViewingSchedule};
use db::repositories::RealityPortalRepository;
use uuid::Uuid;

/// Inquiry validation result.
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
}

/// Validation error details.
#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Inquiry operation result.
#[derive(Debug)]
pub enum InquiryResult {
    /// Inquiry created successfully
    Created(Box<ListingInquiry>),
    /// Validation failed
    ValidationFailed(Vec<ValidationError>),
    /// Listing not found
    ListingNotFound,
    /// Realtor not found
    RealtorNotFound,
    /// Rate limited (too many inquiries)
    RateLimited,
    /// Database error
    DatabaseError(String),
}

/// Inquiries handler for managing contact requests and viewings.
#[derive(Clone)]
pub struct InquiriesHandler {
    repo: RealityPortalRepository,
}

impl InquiriesHandler {
    /// Create a new InquiriesHandler.
    pub fn new(repo: RealityPortalRepository) -> Self {
        Self { repo }
    }

    /// Validate inquiry contact information.
    pub fn validate_contact(
        name: &str,
        email: &str,
        phone: Option<&str>,
        message: &str,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        // Validate name
        let name = name.trim();
        if name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Name is required".to_string(),
            });
        } else if name.len() < 2 {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Name must be at least 2 characters".to_string(),
            });
        } else if name.len() > 100 {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Name must be less than 100 characters".to_string(),
            });
        }

        // Validate email
        let email = email.trim().to_lowercase();
        if email.is_empty() {
            errors.push(ValidationError {
                field: "email".to_string(),
                message: "Email is required".to_string(),
            });
        } else if !Self::is_valid_email(&email) {
            errors.push(ValidationError {
                field: "email".to_string(),
                message: "Invalid email format".to_string(),
            });
        }

        // Validate phone (optional but must be valid if provided)
        if let Some(phone) = phone {
            let phone = phone.trim();
            if !phone.is_empty() && !Self::is_valid_phone(phone) {
                errors.push(ValidationError {
                    field: "phone".to_string(),
                    message: "Invalid phone number format".to_string(),
                });
            }
        }

        // Validate message
        let message = message.trim();
        if message.is_empty() {
            errors.push(ValidationError {
                field: "message".to_string(),
                message: "Message is required".to_string(),
            });
        } else if message.len() < 10 {
            errors.push(ValidationError {
                field: "message".to_string(),
                message: "Message must be at least 10 characters".to_string(),
            });
        } else if message.len() > 2000 {
            errors.push(ValidationError {
                field: "message".to_string(),
                message: "Message must be less than 2000 characters".to_string(),
            });
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        }
    }

    /// Check if email format is valid.
    fn is_valid_email(email: &str) -> bool {
        if email.is_empty() || email.len() > 254 {
            return false;
        }

        if let Some(at_pos) = email.find('@') {
            let local = &email[..at_pos];
            let domain = &email[at_pos + 1..];

            !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        } else {
            false
        }
    }

    /// Check if phone number format is valid.
    fn is_valid_phone(phone: &str) -> bool {
        // Remove common separators and check if remaining chars are digits or +
        let cleaned: String = phone
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '(' && *c != ')')
            .collect();

        if cleaned.is_empty() || cleaned.len() < 9 || cleaned.len() > 15 {
            return false;
        }

        // Must start with + or digit
        let first = cleaned.chars().next().unwrap();
        if first != '+' && !first.is_ascii_digit() {
            return false;
        }

        // Rest must be digits
        cleaned[1..].chars().all(|c| c.is_ascii_digit())
    }

    /// Translate a per-IP / per-listing rate-limit decision into the
    /// short-circuit [`InquiryResult::RateLimited`].
    ///
    /// Extracted as a pure associated fn (no `&self`, no DB) so the throttle
    /// path is unit-testable without a DB-backed handler instance, and so the
    /// public POST routes can enforce the same abuse control they mount the
    /// limiter for. Returns `Some(RateLimited)` when the caller must answer
    /// HTTP 429, `None` when the request may proceed.
    pub fn rate_limit_result(decision: RateLimitDecision) -> Option<InquiryResult> {
        match decision {
            RateLimitDecision::DenyTooManyRequests => Some(InquiryResult::RateLimited),
            RateLimitDecision::Allow => None,
        }
    }

    /// Create a new inquiry.
    ///
    /// The per-IP anonymous throttle is enforced at the ROUTE layer
    /// (`routes::inquiries::enforce_inquiry_rate_limit`, which calls
    /// [`Self::rate_limit_result`]) BEFORE the request ever reaches a
    /// create path, so a rejected flood never touches validation or the DB.
    /// This method therefore takes no rate-limit decision — threading one
    /// through here would be dead code (the live routes call the repository
    /// directly and never construct an `InquiriesHandler`).
    pub async fn create_inquiry(
        &self,
        listing_id: Uuid,
        realtor_id: Uuid,
        user_id: Option<Uuid>,
        data: CreateListingInquiry,
    ) -> InquiryResult {
        // Validate contact info
        let validation = Self::validate_contact(
            &data.name,
            &data.email,
            data.phone.as_deref(),
            &data.message,
        );

        if !validation.is_valid {
            return InquiryResult::ValidationFailed(validation.errors);
        }

        // Create inquiry
        match self
            .repo
            .create_inquiry(listing_id, realtor_id, user_id, data)
            .await
        {
            Ok(inquiry) => {
                // Send notification email (async, don't block)
                Self::send_inquiry_notification(&inquiry).await;
                InquiryResult::Created(Box::new(inquiry))
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("violates foreign key") {
                    if error_str.contains("listing") {
                        InquiryResult::ListingNotFound
                    } else {
                        InquiryResult::RealtorNotFound
                    }
                } else {
                    InquiryResult::DatabaseError(error_str)
                }
            }
        }
    }

    /// Get inquiries for a realtor.
    pub async fn get_realtor_inquiries(
        &self,
        realtor_id: Uuid,
        status: Option<String>,
        page: i32,
        limit: i32,
    ) -> Result<Vec<ListingInquiry>, String> {
        let offset = (page - 1) * limit;
        self.repo
            .get_realtor_inquiries(realtor_id, status, limit, offset)
            .await
            .map_err(|e| e.to_string())
    }

    /// Mark inquiry as read, scoped to its owning realtor.
    ///
    /// Returns `Ok(true)` if the row existed and belonged to the realtor,
    /// `Ok(false)` otherwise. Issue #519: previously the unscoped repo call
    /// was kept here in dead-code form, preserving the IDOR signature for
    /// the next person to wire it up.
    pub async fn mark_as_read(&self, inquiry_id: Uuid, realtor_id: Uuid) -> Result<bool, String> {
        self.repo
            .mark_inquiry_read_for_realtor(inquiry_id, realtor_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Respond to an inquiry.
    pub async fn respond(
        &self,
        inquiry_id: Uuid,
        realtor_id: Uuid,
        message: &str,
    ) -> Result<db::models::InquiryMessage, String> {
        // Validate message
        let message = message.trim();
        if message.is_empty() {
            return Err("Message is required".to_string());
        }
        if message.len() > 5000 {
            return Err("Message must be less than 5000 characters".to_string());
        }

        let response = self
            .repo
            .respond_to_inquiry(inquiry_id, realtor_id, message)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Inquiry not found".to_string())?;

        // Send notification to inquirer (async)
        Self::send_response_notification(inquiry_id).await;

        Ok(response)
    }

    /// Send notification email for new inquiry.
    async fn send_inquiry_notification(inquiry: &ListingInquiry) {
        // In production, this would send an email via email service
        tracing::info!(
            inquiry_id = %inquiry.id,
            realtor_id = %inquiry.realtor_id,
            listing_id = %inquiry.listing_id,
            "Sending inquiry notification email"
        );

        // Email would include:
        // - Inquirer's name, email, phone
        // - Message content
        // - Listing details
        // - Link to respond
    }

    /// Send notification email for inquiry response.
    async fn send_response_notification(inquiry_id: Uuid) {
        // In production, this would send an email to the inquirer
        tracing::info!(
            inquiry_id = %inquiry_id,
            "Sending response notification email"
        );

        // Email would include:
        // - Realtor's response
        // - Link to view conversation
        // - Listing details
    }

    /// Send reminder email for upcoming viewing.
    pub async fn send_viewing_reminder(viewing: &ViewingSchedule) {
        // In production, this would send reminder emails to both parties
        tracing::info!(
            viewing_id = %viewing.id,
            scheduled_at = %viewing.scheduled_at,
            "Sending viewing reminder"
        );

        // Email would include:
        // - Viewing date/time
        // - Property address
        // - Contact details
        // - Cancellation instructions
    }
}

/// Inquiry type constants.
pub mod inquiry_types {
    pub const INFO: &str = "info";
    pub const VIEWING: &str = "viewing";
    pub const OFFER: &str = "offer";
}

/// Inquiry status constants.
pub mod inquiry_status {
    pub const NEW: &str = "new";
    pub const READ: &str = "read";
    pub const RESPONDED: &str = "responded";
    pub const CLOSED: &str = "closed";
}

/// Preferred contact method constants.
pub mod preferred_contact {
    pub const EMAIL: &str = "email";
    pub const PHONE: &str = "phone";
    pub const WHATSAPP: &str = "whatsapp";
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_core::middleware::TenantRateLimiterSet;

    /// IG3 regression: a rate-limit denial constructs `InquiryResult::RateLimited`
    /// (the variant was previously dead — never constructed nor matched).
    #[test]
    fn deny_decision_constructs_rate_limited() {
        assert!(matches!(
            InquiriesHandler::rate_limit_result(RateLimitDecision::DenyTooManyRequests),
            Some(InquiryResult::RateLimited)
        ));
    }

    /// An allowed decision does not short-circuit the create flow.
    #[test]
    fn allow_decision_lets_request_through() {
        assert!(InquiriesHandler::rate_limit_result(RateLimitDecision::Allow).is_none());
    }

    /// IG3 regression, end-to-end on the reused primitive: bursting the same
    /// per-IP key through the real `TenantRateLimiterSet` eventually yields a
    /// denial that maps to `InquiryResult::RateLimited` — proving the public
    /// inquiry POST throttle returns 429 under flood instead of persisting rows.
    #[tokio::test]
    async fn burst_on_one_key_trips_rate_limited() {
        // Low quota so the burst is exhausted quickly and deterministically.
        let limiter = TenantRateLimiterSet::with_default(5);
        let ip_key = Uuid::from_u64_pair(0xABCD, 0x1234);

        let mut tripped = false;
        for _ in 0..50 {
            let decision = limiter.check(ip_key).await;
            if let Some(InquiryResult::RateLimited) = InquiriesHandler::rate_limit_result(decision)
            {
                tripped = true;
                break;
            }
        }
        assert!(
            tripped,
            "a per-key burst must eventually construct InquiryResult::RateLimited"
        );

        // Isolation: a different IP key is unaffected by the first key's flood.
        let fresh_key = Uuid::from_u64_pair(0xABCD, 0x5678);
        assert!(
            InquiriesHandler::rate_limit_result(limiter.check(fresh_key).await).is_none(),
            "a different IP bucket must still be allowed"
        );
    }
}

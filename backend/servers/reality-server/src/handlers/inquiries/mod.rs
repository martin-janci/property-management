//! Inquiry handlers - contact forms and viewing requests.
//!
//! Implements contact validation, best-effort out-of-band notifications,
//! and viewing scheduling functionality.
//!
//! Notifications go through the [`InquiryNotifier`] seam. reality-server has no
//! real email/push transport wired yet, so the default is a logging stub
//! ([`LogInquiryNotifier`]) — the same injectable-transport approach already
//! used by the saved-search alert drainer
//! (`services::search_alert_drainer::AlertEmailTransport`). Delivery is
//! best-effort: a transport failure is logged and never fails the inquiry,
//! which is already persisted and also visible in-app via the realtor's
//! inquiry list.

use api_core::middleware::RateLimitDecision;
use async_trait::async_trait;
use db::models::{CreateListingInquiry, ListingInquiry, ViewingSchedule};
use db::repositories::RealityPortalRepository;
use std::sync::Arc;
use uuid::Uuid;

/// Out-of-band notification transport for inquiry events (new contact request,
/// realtor response, viewing reminder).
///
/// The default is a logging stub ([`LogInquiryNotifier`]); a real email/push
/// adapter can be injected via [`InquiriesHandler::with_notifier`]. This mirrors
/// the transport-trait seam already used by the saved-search alert drainer
/// (`services::search_alert_drainer::AlertEmailTransport`), which exists for the
/// same reason: reality-server has no real email service wired yet.
///
/// All methods are **best-effort**. An `Err` is a delivery failure that the
/// caller logs and swallows — it must never fail the originating request, since
/// the inquiry/response is already persisted.
#[async_trait]
pub trait InquiryNotifier: Send + Sync {
    /// Notify the realtor that a new inquiry (contact/viewing request) arrived.
    async fn notify_new_inquiry(&self, inquiry: &ListingInquiry) -> Result<(), String>;

    /// Notify the inquirer that the realtor responded.
    async fn notify_inquiry_response(&self, inquiry_id: Uuid) -> Result<(), String>;

    /// Remind both parties of an upcoming viewing.
    async fn notify_viewing_reminder(&self, viewing: &ViewingSchedule) -> Result<(), String>;
}

/// Logging stub notifier — records what *would* be sent. Used until a real
/// email/push transport is wired into reality-server.
///
/// TODO(code-review-reality-server-inquiry-email-stub): replace the stub with a
/// real transport. When reality-server gains an email/notification service, add
/// an adapter implementing [`InquiryNotifier`] and inject it via
/// [`InquiriesHandler::with_notifier`] — no handler logic needs to change. Note
/// that the public contact endpoint (`routes::inquiries::send_contact_message`)
/// currently persists via the repository directly and does not route through
/// [`InquiriesHandler`]; wiring it through this handler is the companion
/// follow-up so realtors actually receive the notification promised by the
/// endpoint's "the realtor will respond soon" response.
pub struct LogInquiryNotifier;

#[async_trait]
impl InquiryNotifier for LogInquiryNotifier {
    async fn notify_new_inquiry(&self, inquiry: &ListingInquiry) -> Result<(), String> {
        // A real transport would email the realtor: inquirer name/email/phone,
        // message content, listing details, and a link to respond.
        tracing::info!(
            target: "reality.inquiries",
            inquiry_id = %inquiry.id,
            realtor_id = %inquiry.realtor_id,
            listing_id = %inquiry.listing_id,
            "(stub) new-inquiry notification dispatched",
        );
        Ok(())
    }

    async fn notify_inquiry_response(&self, inquiry_id: Uuid) -> Result<(), String> {
        // A real transport would email the inquirer: the realtor's response, a
        // link to view the conversation, and listing details.
        tracing::info!(
            target: "reality.inquiries",
            inquiry_id = %inquiry_id,
            "(stub) inquiry-response notification dispatched",
        );
        Ok(())
    }

    async fn notify_viewing_reminder(&self, viewing: &ViewingSchedule) -> Result<(), String> {
        // A real transport would email both parties: viewing date/time, property
        // address, contact details, and cancellation instructions.
        tracing::info!(
            target: "reality.inquiries",
            viewing_id = %viewing.id,
            scheduled_at = %viewing.scheduled_at,
            "(stub) viewing-reminder notification dispatched",
        );
        Ok(())
    }
}

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
    notifier: Arc<dyn InquiryNotifier>,
}

impl InquiriesHandler {
    /// Create a new InquiriesHandler with the default logging-stub notifier.
    pub fn new(repo: RealityPortalRepository) -> Self {
        Self {
            repo,
            notifier: Arc::new(LogInquiryNotifier),
        }
    }

    /// Create a new InquiriesHandler with an injected notifier (for a real
    /// email/push transport, or a fake in tests).
    pub fn with_notifier(
        repo: RealityPortalRepository,
        notifier: Arc<dyn InquiryNotifier>,
    ) -> Self {
        Self { repo, notifier }
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
        } else if name.chars().count() < 2 {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Name must be at least 2 characters".to_string(),
            });
        } else if name.chars().count() > 100 {
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
        } else if message.chars().count() < 10 {
            errors.push(ValidationError {
                field: "message".to_string(),
                message: "Message must be at least 10 characters".to_string(),
            });
        } else if message.chars().count() > 2000 {
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
                // Best-effort notification; failure must not fail the inquiry.
                Self::dispatch_new_inquiry(self.notifier.as_ref(), &inquiry).await;
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
        if message.chars().count() > 5000 {
            return Err("Message must be less than 5000 characters".to_string());
        }

        let response = self
            .repo
            .respond_to_inquiry(inquiry_id, realtor_id, message)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Inquiry not found".to_string())?;

        // Best-effort notification to the inquirer; failure must not fail the response.
        Self::dispatch_inquiry_response(self.notifier.as_ref(), inquiry_id).await;

        Ok(response)
    }

    /// Best-effort: dispatch the new-inquiry notification through `notifier`.
    /// A transport failure is logged and swallowed — the inquiry is already
    /// persisted, so notification delivery must never fail the request.
    async fn dispatch_new_inquiry(notifier: &dyn InquiryNotifier, inquiry: &ListingInquiry) {
        if let Err(err) = notifier.notify_new_inquiry(inquiry).await {
            tracing::warn!(
                target: "reality.inquiries",
                inquiry_id = %inquiry.id,
                realtor_id = %inquiry.realtor_id,
                error = %err,
                "new-inquiry notification failed; inquiry is persisted, delivery not retried",
            );
        }
    }

    /// Best-effort: dispatch the inquiry-response notification through `notifier`.
    async fn dispatch_inquiry_response(notifier: &dyn InquiryNotifier, inquiry_id: Uuid) {
        if let Err(err) = notifier.notify_inquiry_response(inquiry_id).await {
            tracing::warn!(
                target: "reality.inquiries",
                inquiry_id = %inquiry_id,
                error = %err,
                "inquiry-response notification failed; response is persisted, delivery not retried",
            );
        }
    }

    /// Best-effort: dispatch a viewing reminder to both parties through the
    /// handler's notifier.
    pub async fn send_viewing_reminder(&self, viewing: &ViewingSchedule) {
        if let Err(err) = self.notifier.notify_viewing_reminder(viewing).await {
            tracing::warn!(
                target: "reality.inquiries",
                viewing_id = %viewing.id,
                error = %err,
                "viewing-reminder notification failed; delivery not retried",
            );
        }
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
    use chrono::Utc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    /// Fake notifier that counts successful `notify_new_inquiry` calls.
    struct CountingNotifier {
        new_inquiry_calls: AtomicUsize,
    }

    #[async_trait]
    impl InquiryNotifier for CountingNotifier {
        async fn notify_new_inquiry(&self, _inquiry: &ListingInquiry) -> Result<(), String> {
            self.new_inquiry_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn notify_inquiry_response(&self, _inquiry_id: Uuid) -> Result<(), String> {
            Ok(())
        }
        async fn notify_viewing_reminder(&self, _viewing: &ViewingSchedule) -> Result<(), String> {
            Ok(())
        }
    }

    /// Fake notifier whose every method fails — models a down transport.
    struct FailingNotifier;

    #[async_trait]
    impl InquiryNotifier for FailingNotifier {
        async fn notify_new_inquiry(&self, _inquiry: &ListingInquiry) -> Result<(), String> {
            Err("transport unavailable".to_string())
        }
        async fn notify_inquiry_response(&self, _inquiry_id: Uuid) -> Result<(), String> {
            Err("transport unavailable".to_string())
        }
        async fn notify_viewing_reminder(&self, _viewing: &ViewingSchedule) -> Result<(), String> {
            Err("transport unavailable".to_string())
        }
    }

    fn sample_inquiry() -> ListingInquiry {
        ListingInquiry {
            id: Uuid::new_v4(),
            listing_id: Uuid::new_v4(),
            realtor_id: Uuid::new_v4(),
            user_id: None,
            name: "Jane Buyer".to_string(),
            email: "jane@example.com".to_string(),
            phone: None,
            message: "Is this still available?".to_string(),
            inquiry_type: inquiry_types::INFO.to_string(),
            preferred_contact: preferred_contact::EMAIL.to_string(),
            preferred_time: None,
            status: inquiry_status::NEW.to_string(),
            read_at: None,
            responded_at: None,
            source: None,
            utm_source: None,
            utm_medium: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn dispatch_new_inquiry_invokes_notifier() {
        let notifier = CountingNotifier {
            new_inquiry_calls: AtomicUsize::new(0),
        };
        InquiriesHandler::dispatch_new_inquiry(&notifier, &sample_inquiry()).await;
        assert_eq!(notifier.new_inquiry_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dispatch_new_inquiry_swallows_transport_failure() {
        // Must complete without panicking or propagating the error: the inquiry
        // is already persisted, so notification delivery is best-effort.
        InquiriesHandler::dispatch_new_inquiry(&FailingNotifier, &sample_inquiry()).await;
    }

    #[tokio::test]
    async fn dispatch_inquiry_response_swallows_transport_failure() {
        InquiriesHandler::dispatch_inquiry_response(&FailingNotifier, Uuid::new_v4()).await;
    }

    #[tokio::test]
    async fn log_stub_notifier_reports_success() {
        let notifier = LogInquiryNotifier;
        assert!(notifier.notify_new_inquiry(&sample_inquiry()).await.is_ok());
        assert!(notifier
            .notify_inquiry_response(Uuid::new_v4())
            .await
            .is_ok());
    }
}

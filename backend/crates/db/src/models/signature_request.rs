//! Signature request models for e-signature integration (Story 7B.3).
//!
//! This module provides types for managing electronic signature workflows
//! on documents, supporting multiple signers with sequential or parallel signing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Status of a signature request.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "signature_request_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SignatureRequestStatus {
    /// Request created, waiting for signers
    #[default]
    Pending,
    /// At least one signer has signed
    InProgress,
    /// All signers have signed
    Completed,
    /// A signer declined to sign
    Declined,
    /// Request expired before completion
    Expired,
    /// Request was cancelled by requester
    Cancelled,
}

impl std::fmt::Display for SignatureRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Declined => write!(f, "declined"),
            Self::Expired => write!(f, "expired"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl SignatureRequestStatus {
    /// Whether this status is terminal (finalized) — the request has reached
    /// an outcome that must never be re-transitioned.
    ///
    /// E-signature providers deliver webhooks at-least-once, so a single
    /// `completed` / `declined` event may arrive multiple times. Once a
    /// request is in a terminal state, replaying a webhook must be a no-op:
    /// it must not re-store the signed document, re-fire requester emails, or
    /// flip signer status back and forth. Webhook handlers consult this guard
    /// before applying any side effect (bug-esignature-webhook-idempotency-guard,
    /// Story 84.2).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Declined | Self::Expired | Self::Cancelled
        )
    }
}

/// Status of an individual signer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SignerStatus {
    /// Waiting for signature
    #[default]
    Pending,
    /// Email sent, awaiting response
    Sent,
    /// Signer viewed the document
    Viewed,
    /// Signer completed their signature
    Signed,
    /// Signer declined to sign
    Declined,
}

impl std::fmt::Display for SignerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Sent => write!(f, "sent"),
            Self::Viewed => write!(f, "viewed"),
            Self::Signed => write!(f, "signed"),
            Self::Declined => write!(f, "declined"),
        }
    }
}

/// Individual signer in a signature request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Signer {
    /// Signer's email address
    pub email: String,
    /// Signer's display name
    pub name: String,
    /// Signing order (for sequential signing)
    #[serde(default)]
    pub order: i32,
    /// Current status of this signer
    #[serde(default)]
    pub status: SignerStatus,
    /// When the signer completed their signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_at: Option<DateTime<Utc>>,
    /// When the signer declined (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declined_at: Option<DateTime<Utc>>,
    /// Reason for declining (if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declined_reason: Option<String>,
    /// External provider's signer ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_signer_id: Option<String>,
    /// When this signer was last sent a reminder email. Consulted by the
    /// scheduler to enforce a minimum interval between reminders and avoid
    /// duplicate sends on every tick. JSONB-backed — older rows that
    /// pre-date this field decode with `None`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_reminder_at: Option<DateTime<Utc>>,
}

impl Signer {
    /// Create a new signer.
    pub fn new(email: String, name: String, order: i32) -> Self {
        Self {
            email,
            name,
            order,
            status: SignerStatus::Pending,
            signed_at: None,
            declined_at: None,
            declined_reason: None,
            provider_signer_id: None,
            last_reminder_at: None,
        }
    }

    /// Check if the signer has completed (signed or declined).
    pub fn is_complete(&self) -> bool {
        matches!(self.status, SignerStatus::Signed | SignerStatus::Declined)
    }

    /// Check if the signer has signed.
    pub fn has_signed(&self) -> bool {
        matches!(self.status, SignerStatus::Signed)
    }
}

/// Signature request entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SignatureRequest {
    /// Unique identifier
    pub id: Uuid,
    /// Document being signed
    pub document_id: Uuid,
    /// Organization context
    pub organization_id: Uuid,
    /// Current request status
    pub status: SignatureRequestStatus,
    /// Email subject/title for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Message to signers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Signers as JSONB
    #[sqlx(json)]
    pub signers: Vec<Signer>,
    /// E-signature provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// External provider's request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    /// Additional provider-specific data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
    /// Signed document (when completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_document_id: Option<Uuid>,
    /// User who created the request
    pub created_by: Uuid,
    /// When the request expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// When all signers completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl SignatureRequest {
    /// Get signers in signing order.
    pub fn signers_in_order(&self) -> Vec<&Signer> {
        let mut signers: Vec<_> = self.signers.iter().collect();
        signers.sort_by_key(|s| s.order);
        signers
    }

    /// Get the next signer who needs to sign.
    pub fn next_signer(&self) -> Option<&Signer> {
        self.signers_in_order()
            .into_iter()
            .find(|s| !s.is_complete())
    }

    /// Check if all signers have signed.
    pub fn all_signed(&self) -> bool {
        !self.signers.is_empty() && self.signers.iter().all(|s| s.has_signed())
    }

    /// Check if any signer has declined.
    pub fn any_declined(&self) -> bool {
        self.signers
            .iter()
            .any(|s| matches!(s.status, SignerStatus::Declined))
    }

    /// Count signers by status.
    pub fn signer_counts(&self) -> SignerCounts {
        let mut counts = SignerCounts::default();
        for signer in &self.signers {
            counts.total += 1;
            match signer.status {
                SignerStatus::Pending => counts.pending += 1,
                SignerStatus::Sent => counts.sent += 1,
                SignerStatus::Viewed => counts.viewed += 1,
                SignerStatus::Signed => counts.signed += 1,
                SignerStatus::Declined => counts.declined += 1,
            }
        }
        counts
    }

    /// Check if the request is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| exp < Utc::now())
    }

    /// Check if the request can be cancelled.
    pub fn can_cancel(&self) -> bool {
        matches!(
            self.status,
            SignatureRequestStatus::Pending | SignatureRequestStatus::InProgress
        )
    }
}

/// Signer count summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SignerCounts {
    pub total: i32,
    pub pending: i32,
    pub sent: i32,
    pub viewed: i32,
    pub signed: i32,
    pub declined: i32,
}

/// Request to create a signature request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSignatureRequest {
    /// Signers for the document
    pub signers: Vec<CreateSigner>,
    /// Email subject/title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Message to signers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// E-signature provider to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Days until request expires (default: 30)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_days: Option<i32>,
}

/// Signer definition for create request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSigner {
    /// Signer's email address
    pub email: String,
    /// Signer's display name
    pub name: String,
    /// Signing order (for sequential signing, default: 0 = parallel)
    #[serde(default)]
    pub order: i32,
}

impl From<CreateSigner> for Signer {
    fn from(cs: CreateSigner) -> Self {
        Signer::new(cs.email, cs.name, cs.order)
    }
}

/// Response after creating a signature request.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateSignatureRequestResponse {
    /// Created signature request
    pub signature_request: SignatureRequest,
    /// Message
    pub message: String,
}

/// Response with signature request details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SignatureRequestResponse {
    /// Signature request details
    pub signature_request: SignatureRequest,
    /// Signer counts summary
    pub signer_counts: SignerCounts,
}

/// Response with list of signature requests.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListSignatureRequestsResponse {
    /// List of signature requests
    pub signature_requests: Vec<SignatureRequest>,
    /// Total count
    pub total: i64,
}

/// Request to send reminder to pending signers.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SendReminderRequest {
    /// Specific signer emails to remind (empty = all pending)
    #[serde(default)]
    pub signer_emails: Vec<String>,
    /// Custom reminder message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Response after sending reminders.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SendReminderResponse {
    /// Number of reminders sent
    pub reminders_sent: i32,
    /// Message
    pub message: String,
}

/// Request to cancel a signature request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CancelSignatureRequestRequest {
    /// Reason for cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response after cancelling.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CancelSignatureRequestResponse {
    /// Updated signature request
    pub signature_request: SignatureRequest,
    /// Message
    pub message: String,
}

/// Webhook event from signature provider.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SignatureWebhookEvent {
    /// Event type
    pub event_type: String,
    /// Provider request ID
    pub provider_request_id: String,
    /// Signer email (if signer-specific event)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_email: Option<String>,
    /// New status for the signer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_status: Option<SignerStatus>,
    /// Decline reason (if declined)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decline_reason: Option<String>,
    /// Signed document URL (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_document_url: Option<String>,
    /// Raw event payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_payload: Option<serde_json::Value>,
}

/// Response after processing webhook.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebhookResponse {
    /// Whether the webhook was processed successfully
    pub success: bool,
    /// Message
    pub message: String,
}

/// Signature request with document details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SignatureRequestWithDocument {
    /// Signature request
    #[serde(flatten)]
    pub request: SignatureRequest,
    /// Document name
    pub document_name: String,
    /// Document file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_file_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_new() {
        let signer = Signer::new("test@example.com".into(), "Test User".into(), 1);
        assert_eq!(signer.email, "test@example.com");
        assert_eq!(signer.name, "Test User");
        assert_eq!(signer.order, 1);
        assert!(matches!(signer.status, SignerStatus::Pending));
        assert!(!signer.is_complete());
        assert!(!signer.has_signed());
    }

    #[test]
    fn test_signer_is_complete() {
        let mut signer = Signer::new("test@example.com".into(), "Test".into(), 0);

        signer.status = SignerStatus::Pending;
        assert!(!signer.is_complete());

        signer.status = SignerStatus::Signed;
        assert!(signer.is_complete());
        assert!(signer.has_signed());

        signer.status = SignerStatus::Declined;
        assert!(signer.is_complete());
        assert!(!signer.has_signed());
    }

    #[test]
    fn test_status_is_terminal() {
        // Terminal (finalized) states — webhook replays must be no-ops.
        assert!(SignatureRequestStatus::Completed.is_terminal());
        assert!(SignatureRequestStatus::Declined.is_terminal());
        assert!(SignatureRequestStatus::Expired.is_terminal());
        assert!(SignatureRequestStatus::Cancelled.is_terminal());

        // Non-terminal states — still progressing, webhooks may transition.
        assert!(!SignatureRequestStatus::Pending.is_terminal());
        assert!(!SignatureRequestStatus::InProgress.is_terminal());
    }

    #[test]
    fn test_create_signer_to_signer() {
        let create = CreateSigner {
            email: "test@example.com".into(),
            name: "Test User".into(),
            order: 2,
        };
        let signer: Signer = create.into();
        assert_eq!(signer.email, "test@example.com");
        assert_eq!(signer.order, 2);
    }
}

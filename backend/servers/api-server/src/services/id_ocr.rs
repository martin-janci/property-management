//! Guest ID-document OCR seam (Epic 18, Story 18.2 — issue #1687).
//!
//! Stage A wires a **provider-agnostic** seam: an [`IdDocumentOcr`] trait, a
//! [`GuestIdExtraction`] DTO, and a [`StubIdDocumentOcr`] that fails closed with
//! [`IdOcrError::NotConfigured`]. The upload + extract endpoints in
//! `routes/rentals.rs` call through this trait so the real provider can be
//! dropped in later (Stage B) without touching the route layer.
//!
//! Stage B (a real Vision-capable OCR/LLM provider) is a follow-up that needs an
//! external integration decision — the text-only `LlmClient` cannot read images.
//! Until then the extract endpoint degrades gracefully to `501
//! OCR_NOT_CONFIGURED` (mirroring `routes/ai/ocr.rs`).

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Structured fields an OCR provider may extract from a guest ID document.
///
/// Every field is optional: a provider returns only what it could confidently
/// read, and the manager confirms/edits the values via the existing
/// `PUT /api/v1/rentals/guests/{id}` flow (extraction is **never** auto-applied).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GuestIdExtraction {
    /// Given name(s) as printed on the document.
    pub first_name: Option<String>,
    /// Family name as printed on the document.
    pub last_name: Option<String>,
    /// Date of birth (`YYYY-MM-DD`).
    pub date_of_birth: Option<String>,
    /// Nationality (ISO 3166-1 alpha-3 where available).
    pub nationality: Option<String>,
    /// Document type, e.g. `passport`, `national_id`, `driving_license`.
    pub id_type: Option<String>,
    /// Document number.
    pub id_number: Option<String>,
    /// Issuing country (ISO 3166-1 alpha-3 where available).
    pub id_issuing_country: Option<String>,
    /// Document expiry date (`YYYY-MM-DD`).
    pub id_expiry_date: Option<String>,
    /// Provider confidence in the extraction, `0.0..=1.0` when reported.
    pub confidence: Option<f64>,
}

/// Errors an [`IdDocumentOcr`] provider may return.
#[derive(Debug, thiserror::Error)]
pub enum IdOcrError {
    /// No OCR provider is configured (the Stage-A stub). Routes map this to
    /// `501 OCR_NOT_CONFIGURED` so the frontend can degrade gracefully.
    #[error("OCR provider is not configured")]
    NotConfigured,

    /// The provider rejected the document (unsupported/corrupt/unreadable).
    #[error("OCR provider could not process the document: {0}")]
    InvalidDocument(String),

    /// The provider failed for an operational reason (network, quota, 5xx).
    #[error("OCR provider error: {0}")]
    Provider(String),
}

/// Provider-agnostic guest ID-document OCR.
///
/// Implementors take the raw document bytes + MIME and return a best-effort
/// [`GuestIdExtraction`]. The trait is object-safe so it can live behind an
/// `Arc<dyn ...>` on [`AppState`](crate::state::AppState).
#[async_trait]
pub trait IdDocumentOcr: Send + Sync {
    /// Extract structured guest fields from a single ID document.
    async fn extract(&self, bytes: &[u8], mime: &str) -> Result<GuestIdExtraction, IdOcrError>;
}

/// Shared, cloneable handle to an [`IdDocumentOcr`] provider.
pub type SharedIdDocumentOcr = Arc<dyn IdDocumentOcr + Send + Sync>;

/// Stage-A stub provider: always fails closed with [`IdOcrError::NotConfigured`].
///
/// This is the default wired into [`AppState`](crate::state::AppState). Swapping
/// in a real provider (Stage B) is a one-line change in `state.rs` with no route
/// changes.
#[derive(Debug, Default, Clone)]
pub struct StubIdDocumentOcr;

#[async_trait]
impl IdDocumentOcr for StubIdDocumentOcr {
    async fn extract(&self, _bytes: &[u8], _mime: &str) -> Result<GuestIdExtraction, IdOcrError> {
        Err(IdOcrError::NotConfigured)
    }
}

/// Default provider for [`AppState`](crate::state::AppState): the not-configured
/// stub. Replace this with a real provider once Stage B lands.
pub fn default_id_document_ocr() -> SharedIdDocumentOcr {
    Arc::new(StubIdDocumentOcr)
}

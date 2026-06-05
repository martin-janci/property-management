//! AI routes (Epic 13: AI Assistant & Automation).
//!
//! Handles AI chat, sentiment analysis, equipment/maintenance, and workflows.
//! Epic 91: Wired to actual LLM providers (OpenAI/Anthropic).
//!
//! # Authentication & tenancy (SECURITY-CRITICAL)
//!
//! Every handler in this module derives tenancy from the verified
//! [`RequestPrincipal`] (built from the bearer JWT + host-resolved tenant).
//! The previous implementation read tenancy from a client-supplied
//! `X-Tenant-Context` header, which let any unauthenticated caller forge a
//! tenant id and burn the company's LLM billing on behalf of arbitrary
//! tenants. That extractor has been deleted; see git history for
//! `extract_tenant_context`.
//!
//! # Module layout
//!
//! This module was split from a single ~3.1k-LOC `ai.rs` into surface-scoped
//! sub-modules. The split is behaviour-preserving: every route registration,
//! handler body, authz/RLS guard, and the voice-device-IDOR / equipment-IDOR
//! security fixes are carried over verbatim.
//!
//! | Surface           | Module       | Router fn                          |
//! |-------------------|--------------|------------------------------------|
//! | Chat + sentiment  | `sessions`   | `ai_chat_router`, `sentiment_router` |
//! | Equipment         | `equipment`  | `equipment_router`                 |
//! | Workflows         | `workflows`  | `workflow_router`                  |
//! | LLM document/chat | `llm`        | `llm_router`                       |
//! | Voice assistant   | `voice`      | (handlers mounted by `llm_router`) |

use api_core::extractors::principal::RequestPrincipal;
use axum::{http::StatusCode, Json};
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub mod equipment;
pub mod llm;
pub mod sessions;
pub mod voice;
pub mod workflows;

// Re-export the public router constructors so external references
// (`routes::ai::ai_chat_router()` etc. in `lib.rs`) keep working unchanged.
pub use equipment::equipment_router;
pub use llm::llm_router;
pub use sessions::{ai_chat_router, sentiment_router};
pub use workflows::workflow_router;

// ============================================================================
// Helper Functions
// ============================================================================

/// Resolve the effective tenant id from a verified [`RequestPrincipal`].
///
/// AI endpoints are per-tenant by definition (cost attribution, RAG context,
/// chat history). A platform-kind principal hitting the platform host has no
/// `effective_org` — for those callers we refuse with 403 rather than fall
/// back to a wildcard. Non-platform principals without a host-resolved tenant
/// never reach this point because `RequestPrincipal` rejects them earlier.
pub(crate) fn require_tenant_id(
    principal: &RequestPrincipal,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    principal.effective_org.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "TENANT_REQUIRED",
                "AI endpoints require a tenant-resolved request",
            )),
        )
    })
}

/// Boilerplate 404 for any cross-tenant probe or genuine miss.
///
/// We deliberately use the same response for "doesn't exist" and "exists in
/// another org" so a caller can't enumerate workflow IDs by comparing 403 vs
/// 404. See H1 audit notes.
pub(crate) fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct AlertsQuery {
    pub acknowledged: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

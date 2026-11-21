//! Voice assistant webhook routes (Epic 93: Voice Assistant & OAuth Completion).
//!
//! Story 93.3: Voice Platform Webhooks
//! - Alexa Skills Kit webhook handler
//! - Google Actions webhook handler
//! - Request signature verification
//! - User authentication via OAuth token
//!
//! # Module layout
//!
//! This module was split from a single ~3.1k-LOC `voice_webhooks.rs` into
//! surface-scoped pieces. The split is behaviour-preserving — every route
//! registration, handler body, auth/RLS guard, and the webhook security fixes
//! (#2658 broken-auth, #2668 Alexa forgery, #890 fail-open simulated tokens,
//! #765 fail-closed encryption, #2769 refresh/exchange throttling) are carried
//! over verbatim.
//!
//! | Surface                      | Location                              |
//! |------------------------------|---------------------------------------|
//! | Router                       | this module (`mod.rs`)                |
//! | Platform webhook handlers    | [`handlers`]                          |
//! | OAuth exchange/refresh       | [`oauth`]                             |
//! | JSON `/verify` introspection | [`verify`]                            |
//! | Signature / token verify     | [`signature`]                         |
//! | Provider request/response    | [`parsers`]                           |
//! | OAuth + device auth logic    | [`crate::services::voice_oauth`]      |

mod handlers;
mod oauth;
pub mod parsers;
pub mod signature;
mod verify;

use crate::state::AppState;
use axum::{routing::post, Router};

use handlers::{alexa_health_check, alexa_webhook, google_actions_webhook};
use oauth::{oauth_token_exchange, oauth_token_refresh};
use verify::verify_webhook_signature;

// ============================================================================
// Router
// ============================================================================

/// Voice webhook router for external platform integrations.
pub fn voice_webhook_router() -> Router<AppState> {
    Router::new()
        // Alexa Skills Kit
        .route("/alexa", post(alexa_webhook))
        .route("/alexa/health", post(alexa_health_check))
        // Google Actions
        .route("/google", post(google_actions_webhook))
        // OAuth token exchange endpoints
        .route("/oauth/exchange", post(oauth_token_exchange))
        .route("/oauth/refresh", post(oauth_token_refresh))
        // Verification endpoint
        .route("/verify", post(verify_webhook_signature))
}

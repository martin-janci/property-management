//! Admin MFA enrollment + recovery-code routes. Phase 6 (B6).
//!
//! These endpoints are for *platform principals* only. They do NOT go through
//! the `RequireCapability` gate because enrollment itself is a precondition to
//! that gate — you cannot satisfy MFA if you have never enrolled.
//!
//! Endpoints:
//!   * `POST /admin/mfa/enroll/start`   — generate TOTP secret, return QR URI
//!   * `POST /admin/mfa/enroll/verify`  — confirm code, activate, issue recovery codes
//!   * `POST /admin/mfa/verify`         — TOTP step-up for enrolled users (opens recency window)
//!   * `POST /admin/mfa/recovery/use`   — consume a recovery code (lockout fallback)
//!   * `POST /admin/mfa/disable`        — clear enrollment (requires fresh MFA or recovery code)

mod disable;
mod enroll;
mod recovery;
mod verify;

pub use disable::disable_mfa;
pub use enroll::{start_enroll, verify_enroll};
pub use recovery::use_recovery;
pub use verify::verify_step_up;

use axum::{routing::post, Router};

use crate::state::AppState;

/// Sub-router mounted at `/api/v1/admin/mfa`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/enroll/start", post(start_enroll))
        .route("/enroll/verify", post(verify_enroll))
        .route("/verify", post(verify_step_up))
        .route("/recovery/use", post(use_recovery))
        .route("/disable", post(disable_mfa))
}

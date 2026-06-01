//! Admin MFA step-up (TOTP) verification. Phase 6 (B6) follow-up (#764).
//!
//! `POST /admin/mfa/verify` is the interactive TOTP step-up for an *already
//! enrolled* platform principal. When a capability-gated admin call returns
//! `401 mfa_required`, the admin-web `MfaChallengeProvider` prompts for a
//! 6-digit code and POSTs it here. On success the server records a
//! `two_factor_auth_verifications` row (`method = 'totp'`) so the 15-minute
//! recency window opens and the original mutation can be retried.
//!
//! This is distinct from:
//!   * `POST /api/v1/auth/mfa/verify` — *setup completion* (enables MFA, issues
//!     recovery codes, 409s if already enabled, never opens a recency window).
//!   * `POST /admin/mfa/recovery/use` — lockout fallback via single-use code.
//!
//! Before this endpoint existed an enrolled admin could only satisfy the
//! recency gate by burning a recovery code, which is wrong: recovery codes are
//! a one-time lockout escape hatch, not the day-to-day step-up path.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

use admin_core::{AuditOutcome, AuditWriter, Capability};
use api_core::extractors::principal::RequestPrincipal;

use crate::state::AppState;

/// Request body for `POST /admin/mfa/verify`.
#[derive(Debug, Deserialize)]
pub struct VerifyStepUpRequest {
    /// 6-digit TOTP code from the authenticator app.
    pub code: String,
}

/// Response for `POST /admin/mfa/verify`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyStepUpResponse {
    pub message: String,
}

/// Verify a TOTP code for an enrolled platform principal and open the
/// 15-minute MFA recency window.
///
/// Side-effects on success:
///   1. Inserts a `two_factor_auth_verifications` row (`method = 'totp'`).
///   2. Audits the event.
pub async fn verify_step_up(
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    Json(req): Json<VerifyStepUpRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = principal.user_id;

    // Platform-kind guard. Mirrors the rest of the admin MFA router — the
    // RequireCapability gate is not in play here because satisfying that gate
    // is the entire purpose of this call.
    if !principal.is_platform() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "not_platform_principal",
                "message": "Admin MFA step-up is for platform principals only."
            })),
        ));
    }

    // Load the enrollment + encrypted secret. Must be enabled — an
    // un-enrolled user has no secret to verify against and must enroll first.
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT secret, enabled FROM user_2fa WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "mfa/verify: fetch user_2fa failed");
                internal()
            })?;

    let (encrypted_secret, enabled) = row.ok_or_else(|| {
        (
            StatusCode::PRECONDITION_FAILED,
            Json(serde_json::json!({
                "error": "mfa_not_enrolled",
                "message": "MFA is not enrolled. Complete enrollment before step-up."
            })),
        )
    })?;

    if !enabled {
        return Err((
            StatusCode::PRECONDITION_FAILED,
            Json(serde_json::json!({
                "error": "mfa_not_enrolled",
                "message": "MFA is not enrolled. Complete enrollment before step-up."
            })),
        ));
    }

    // Decrypt + verify the code.
    let secret = state
        .totp_service
        .decrypt_secret(&encrypted_secret)
        .map_err(|e| {
            tracing::error!(error = %e, "mfa/verify: decrypt failed");
            internal()
        })?;

    let valid = state
        .totp_service
        .verify_code(&secret, req.code.trim())
        .unwrap_or(false);

    if !valid {
        let _ = audit
            .record(
                Some(user_id),
                Capability::UsersWrite,
                AuditOutcome::Denied,
                Some("mfa_step_up_invalid"),
                Some(user_id),
                None,
                None,
                None,
            )
            .await;
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": "invalid_code",
                "message": "The TOTP code is invalid. Check your authenticator app and try again."
            })),
        ));
    }

    // Open the 15-minute MFA recency window. Same row shape the recency check
    // (`admin_core::PgMfaRecency` / `capabilities.rs`) reads from.
    sqlx::query("INSERT INTO two_factor_auth_verifications (user_id, method) VALUES ($1, 'totp')")
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "mfa/verify: insert verification failed");
            internal()
        })?;

    let _ = audit
        .record(
            Some(user_id),
            Capability::UsersWrite,
            AuditOutcome::Allowed,
            Some("mfa_step_up"),
            Some(user_id),
            None,
            None,
            None,
        )
        .await;

    tracing::info!(user_id = %user_id, "admin mfa step-up verified");
    Ok(Json(VerifyStepUpResponse {
        message: "MFA verified. The 15-minute verification window is now open.".into(),
    }))
}

fn internal() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "internal" })),
    )
}

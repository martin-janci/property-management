//! Admin MFA recovery-code consumption. Phase 6 (B6).
//!
//! `POST /admin/mfa/recovery/use` is the lockout-escape hatch: when a platform
//! principal has lost access to their TOTP app they may present one of the 10
//! single-use recovery codes issued at enrollment time. On success the server
//! records a `two_factor_auth_verifications` row (same as a normal TOTP verify)
//! so the 15-minute recency window opens and the next capability call succeeds.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

use admin_core::{AuditOutcome, AuditWriter, Capability};
use api_core::extractors::principal::RequestPrincipal;

use crate::state::AppState;

/// Request body for `POST /admin/mfa/recovery/use`.
#[derive(Debug, Deserialize)]
pub struct UseRecoveryRequest {
    /// Plaintext recovery code (as displayed during enrollment).
    pub code: String,
}

/// Response for `POST /admin/mfa/recovery/use`.
#[derive(Debug, Serialize)]
pub struct UseRecoveryResponse {
    pub message: String,
    /// How many unused recovery codes remain after this call.
    pub codes_remaining: i64,
}

/// Consume a single-use recovery code as an alternative to TOTP.
///
/// Side-effects on success:
///   1. Marks the matching `mfa_recovery_codes` row as used (`used_at = NOW()`).
///   2. Inserts a `two_factor_auth_verifications` row (`method = 'recovery_code'`).
///   3. Audits the event.
pub async fn use_recovery(
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    Json(req): Json<UseRecoveryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = principal.user_id;

    if !principal.is_platform() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "not_platform_principal" })),
        ));
    }

    // Verify MFA is enrolled.
    let enrolled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "recovery/use: fetch enrollment failed");
                internal()
            })?;

    if !matches!(enrolled, Some(true)) {
        return Err((
            StatusCode::PRECONDITION_FAILED,
            Json(serde_json::json!({
                "error": "not_enrolled",
                "message": "MFA is not enrolled. Recovery codes are only valid for enrolled users."
            })),
        ));
    }

    // Load unused recovery codes for this user.
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, code_hash FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "recovery/use: fetch codes failed");
        internal()
    })?;

    if rows.is_empty() {
        let _ = audit
            .record(
                Some(user_id),
                Capability::UsersWrite,
                AuditOutcome::Denied,
                Some("mfa_recovery_no_codes"),
                Some(user_id),
                None,
                None,
                None,
            )
            .await;
        return Err((
            StatusCode::GONE,
            Json(serde_json::json!({
                "error": "no_recovery_codes",
                "message": "All recovery codes have been used. Contact your platform administrator."
            })),
        ));
    }

    // Try each unused code with constant-time Argon2 comparison.
    let normalized = req.code.trim().replace(['-', ' '], "").to_uppercase();
    let hashes: Vec<String> = rows.iter().map(|(_, h)| h.clone()).collect();
    let matched_idx = state
        .totp_service
        .verify_backup_code(&normalized, &hashes)
        .map_err(|e| {
            tracing::error!(error = %e, "recovery/use: verify_backup_code failed");
            internal()
        })?;

    let Some(idx) = matched_idx else {
        let _ = audit
            .record(
                Some(user_id),
                Capability::UsersWrite,
                AuditOutcome::Denied,
                Some("mfa_recovery_invalid"),
                Some(user_id),
                None,
                None,
                None,
            )
            .await;
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": "invalid_recovery_code",
                "message": "Recovery code did not match. Check the code and try again."
            })),
        ));
    };

    let matched_id = rows[idx].0;

    // Mark as used — single-use enforced.
    sqlx::query("UPDATE mfa_recovery_codes SET used_at = NOW() WHERE id = $1 AND used_at IS NULL")
        .bind(matched_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "recovery/use: mark used failed");
            internal()
        })?;

    // Open the 15-minute MFA recency window (same as a TOTP verify).
    sqlx::query(
        "INSERT INTO two_factor_auth_verifications (user_id, method) VALUES ($1, 'recovery_code')",
    )
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "recovery/use: insert verification failed");
        internal()
    })?;

    // Count remaining codes.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Audit success.
    let _ = audit
        .record(
            Some(user_id),
            Capability::UsersWrite,
            AuditOutcome::Allowed,
            Some("mfa_recovery_used"),
            Some(user_id),
            None,
            None,
            None,
        )
        .await;

    tracing::info!(user_id = %user_id, remaining, "admin mfa recovery code consumed");
    Ok(Json(UseRecoveryResponse {
        message: "Recovery code accepted. MFA verification window is now open (15 minutes).".into(),
        codes_remaining: remaining,
    }))
}

fn internal() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "internal" })),
    )
}

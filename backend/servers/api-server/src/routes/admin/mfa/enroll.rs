//! Admin MFA enrollment endpoints. Phase 6 (B6).
//!
//! Platform principals call `start` to get a QR code, then `verify` to
//! activate enrollment and receive their 10 one-time recovery codes.
//! The TOTP secret is stored in the existing `user_2fa` table (migration 00024).
//! Recovery codes are stored in `mfa_recovery_codes` (migration 00149).

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

use admin_core::{AuditOutcome, AuditWriter, Capability};
use api_core::extractors::principal::RequestPrincipal;

use crate::state::AppState;

// ── Start enrollment ──────────────────────────────────────────────────────────

/// Response returned by `POST /admin/mfa/enroll/start`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollStartResponse {
    /// `otpauth://` URI — hand to a QR-code library.
    pub uri: String,
    /// Base32 TOTP secret — for users who prefer manual entry.
    pub secret: String,
    /// Base64-encoded PNG of the QR code, generated server-side so the
    /// otpauth URI (which embeds the TOTP secret) never leaves the trust
    /// boundary. Frontend renders via `<img src="data:image/png;base64,...">`.
    pub qr_png_base64: String,
}

/// Generate a fresh TOTP secret and return the enrollment URI.
///
/// **Does not activate MFA**. The secret is stored (encrypted) in `user_2fa`
/// with `enabled = false`; the caller must call `verify` next.
///
/// Safe to call multiple times — each call replaces the pending (non-active)
/// setup.
pub async fn start_enroll(
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = principal.user_id;

    // Platform-kind guard. The RequireCapability gate is not in play here
    // (enrollment is a precondition to it), so we enforce the kind check
    // manually.
    if !principal.is_platform() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "not_platform_principal",
                "message": "Admin MFA enrollment is for platform principals only."
            })),
        ));
    }

    // Fetch user email for the TOTP issuer label.
    let email: Option<String> =
        sqlx::query_scalar("SELECT email FROM users WHERE id = $1 AND status != 'deleted'")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "enroll/start: user email lookup failed");
                internal()
            })?;

    let email = email.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "user_not_found" })),
        )
    })?;

    // Generate fresh secret.
    let secret = state.totp_service.generate_secret().map_err(|e| {
        tracing::error!(error = %e, "enroll/start: generate_secret failed");
        internal()
    })?;

    let uri = state
        .totp_service
        .generate_qr_uri(&email, &secret)
        .map_err(|e| {
            tracing::error!(error = %e, "enroll/start: generate_qr_uri failed");
            internal()
        })?;

    let qr_png_base64 = state
        .totp_service
        .generate_qr_base64(&email, &secret)
        .map_err(|e| {
            tracing::error!(error = %e, "enroll/start: generate_qr_base64 failed");
            internal()
        })?;

    // Reject re-enrollment if MFA is already enabled. Allowing an
    // unauthenticated re-enroll would let a stolen bearer token reset the
    // legitimate user's TOTP secret and lock them out (Copilot 3252772624).
    // To rotate an existing enrollment the user must disable first (which
    // requires the current TOTP code or a recovery code).
    let already_enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "enroll/start: fetch existing enrollment failed");
                internal()
            })?;

    if matches!(already_enabled, Some(true)) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "already_enrolled",
                "message": "MFA is already enabled for this user. Disable existing MFA (with a current TOTP or recovery code) before starting a new enrollment."
            })),
        ));
    }

    // Encrypt and upsert into user_2fa (enabled = false).
    // The CONFLICT branch only fires when an existing pending (non-enabled)
    // setup is being retried — the enabled=true guard above ensures we
    // never silently overwrite an active secret.
    let encrypted = state.totp_service.encrypt_secret(&secret).map_err(|e| {
        tracing::error!(error = %e, "enroll/start: encrypt_secret failed");
        internal()
    })?;

    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, false, '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE
            SET secret = EXCLUDED.secret,
                enabled = false,
                enabled_at = NULL,
                backup_codes = '[]'::jsonb,
                backup_codes_remaining = 0,
                updated_at = NOW()
            WHERE user_2fa.enabled = false
        "#,
    )
    .bind(user_id)
    .bind(&encrypted)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "enroll/start: upsert user_2fa failed");
        internal()
    })?;

    // Audit.
    let _ = audit
        .record(
            Some(user_id),
            Capability::UsersWrite, // closest semantic match for self-enrollment
            AuditOutcome::Allowed,
            Some("mfa_enroll_start"),
            Some(user_id),
            None,
            None,
            None,
        )
        .await;

    tracing::info!(user_id = %user_id, "admin mfa enroll/start");
    Ok(Json(EnrollStartResponse {
        uri,
        secret,
        qr_png_base64,
    }))
}

// ── Verify enrollment ─────────────────────────────────────────────────────────

/// Request body for `POST /admin/mfa/enroll/verify`.
#[derive(Debug, Deserialize)]
pub struct EnrollVerifyRequest {
    /// 6-digit TOTP code from the authenticator app.
    pub code: String,
}

/// Response returned by `POST /admin/mfa/enroll/verify`.
#[derive(Debug, Serialize)]
pub struct EnrollVerifyResponse {
    pub message: String,
    /// 10 plaintext recovery codes — shown ONCE. Tell the user to save them.
    pub recovery_codes: Vec<String>,
}

/// Confirm a TOTP code to activate enrollment and issue 10 recovery codes.
pub async fn verify_enroll(
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    Json(req): Json<EnrollVerifyRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = principal.user_id;

    if !principal.is_platform() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "not_platform_principal" })),
        ));
    }

    // Fetch the pending setup.
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT secret, enabled FROM user_2fa WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "enroll/verify: fetch user_2fa failed");
                internal()
            })?;

    let (encrypted_secret, already_enabled) = row.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "enroll_not_started",
                "message": "Call /enroll/start first."
            })),
        )
    })?;

    if already_enabled {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "already_enrolled",
                "message": "MFA is already active. Disable it first to re-enroll."
            })),
        ));
    }

    // Decrypt + verify code.
    let secret = state
        .totp_service
        .decrypt_secret(&encrypted_secret)
        .map_err(|e| {
            tracing::error!(error = %e, "enroll/verify: decrypt failed");
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
                Some("mfa_enroll_verify"),
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

    // Activate enrollment + rotate recovery codes atomically: enabling 2FA
    // without persisting the codes would lock the operator out if anything
    // fails mid-way (review follow-up — Copilot 3252731618).
    let (plain_codes, hashed_codes) = state.totp_service.generate_backup_codes().map_err(|e| {
        tracing::error!(error = %e, "enroll/verify: generate_backup_codes failed");
        internal()
    })?;

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = %e, "enroll/verify: begin tx failed");
        internal()
    })?;

    sqlx::query(
        "UPDATE user_2fa SET enabled = true, enabled_at = NOW(), updated_at = NOW() WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "enroll/verify: enable user_2fa failed");
        internal()
    })?;

    sqlx::query("DELETE FROM mfa_recovery_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "enroll/verify: delete old recovery codes failed");
            internal()
        })?;

    for hash in &hashed_codes {
        sqlx::query("INSERT INTO mfa_recovery_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "enroll/verify: insert recovery code failed");
                internal()
            })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "enroll/verify: commit tx failed");
        internal()
    })?;

    // Audit success.
    let _ = audit
        .record(
            Some(user_id),
            Capability::UsersWrite,
            AuditOutcome::Allowed,
            Some("mfa_enrolled"),
            Some(user_id),
            None,
            None,
            None,
        )
        .await;

    tracing::info!(user_id = %user_id, "admin mfa enrolled");
    Ok(Json(EnrollVerifyResponse {
        message:
            "MFA enrollment complete. Store your recovery codes — they will not be shown again."
                .into(),
        recovery_codes: plain_codes,
    }))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn internal() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "internal" })),
    )
}

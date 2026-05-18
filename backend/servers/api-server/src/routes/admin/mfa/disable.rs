//! Admin MFA disable endpoint. Phase 6 (B6).
//!
//! `POST /admin/mfa/disable` clears enrollment. Requires either:
//!   * A valid 6-digit TOTP code (fresh authenticator access), OR
//!   * A valid unused recovery code (lockout escape hatch).
//!
//! Success resets `user_2fa` and deletes all `mfa_recovery_codes` rows for
//! the caller.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

use admin_core::{AuditOutcome, AuditWriter, Capability};
use api_core::extractors::principal::RequestPrincipal;

use crate::state::AppState;

/// Request body for `POST /admin/mfa/disable`.
#[derive(Debug, Deserialize)]
pub struct DisableRequest {
    /// Either the 6-digit TOTP code OR a recovery code.
    pub code: String,
}

/// Response for `POST /admin/mfa/disable`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableResponse {
    pub message: String,
}

/// Disable MFA enrollment for the calling platform principal.
pub async fn disable_mfa(
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    Json(req): Json<DisableRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = principal.user_id;

    if !principal.is_platform() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "not_platform_principal" })),
        ));
    }

    // Fetch current enrollment state.
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT secret, enabled FROM user_2fa WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "mfa/disable: fetch user_2fa failed");
                internal()
            })?;

    let (encrypted_secret, enabled) = row.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_enrolled",
                "message": "MFA is not enrolled."
            })),
        )
    })?;

    if !enabled {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_enrolled",
                "message": "MFA is not active."
            })),
        ));
    }

    // Attempt TOTP verification first.
    let secret = state
        .totp_service
        .decrypt_secret(&encrypted_secret)
        .map_err(|e| {
            tracing::error!(error = %e, "mfa/disable: decrypt failed");
            internal()
        })?;

    let totp_ok = state
        .totp_service
        .verify_code(&secret, req.code.trim())
        .unwrap_or(false);

    // Resolve which recovery-code row (if any) the input matched, BEFORE
    // we open a transaction. The hash-compare is in-memory; nothing is
    // written here. We delay the UPDATE until inside the tx so a crash
    // between "consume code" and "disable user_2fa" cannot strand the
    // user with MFA enabled but a code spent (Copilot 3252731626).
    let recovery_match: Option<uuid::Uuid> = if totp_ok {
        None
    } else {
        let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
            "SELECT id, code_hash FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "mfa/disable: fetch recovery codes failed");
            internal()
        })?;

        if rows.is_empty() {
            None
        } else {
            let normalized = req.code.trim().replace(['-', ' '], "").to_uppercase();
            let hashes: Vec<String> = rows.iter().map(|(_, h)| h.clone()).collect();
            state
                .totp_service
                .verify_backup_code(&normalized, &hashes)
                .map_err(|e| {
                    tracing::error!(error = %e, "mfa/disable: verify_backup_code failed");
                    internal()
                })?
                .map(|idx| rows[idx].0)
        }
    };

    let code_ok = totp_ok || recovery_match.is_some();

    if !code_ok {
        let _ = audit
            .record(
                Some(user_id),
                Capability::UsersWrite,
                AuditOutcome::Denied,
                Some("mfa_disable_rejected"),
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
                "message": "Provide your current TOTP code or a valid recovery code."
            })),
        ));
    }

    // Atomic: mark recovery code used (if that was the verification method),
    // disable user_2fa, and purge all recovery codes — all-or-nothing so a
    // crash mid-flow cannot strand the user.
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = %e, "mfa/disable: begin tx failed");
        internal()
    })?;

    if let Some(rid) = recovery_match {
        // Single-use guard: AND used_at IS NULL ensures a concurrent caller
        // racing the same code is rejected here too. rows_affected == 0
        // means another request already consumed it — treat as invalid
        // (rollback) and ask the user to retry.
        let upd = sqlx::query(
            "UPDATE mfa_recovery_codes SET used_at = NOW() \
             WHERE id = $1 AND used_at IS NULL",
        )
        .bind(rid)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "mfa/disable: mark recovery used failed");
            internal()
        })?;
        if upd.rows_affected() != 1 {
            // Tx will roll back on drop. Surface as invalid code.
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "invalid_code",
                    "message": "Recovery code was used concurrently. Try again with a different code."
                })),
            ));
        }
    }

    sqlx::query(
        "UPDATE user_2fa SET enabled = false, enabled_at = NULL, updated_at = NOW() WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "mfa/disable: update user_2fa failed");
        internal()
    })?;

    sqlx::query("DELETE FROM mfa_recovery_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "mfa/disable: delete recovery codes failed");
            internal()
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "mfa/disable: commit tx failed");
        internal()
    })?;

    // Audit.
    let _ = audit
        .record(
            Some(user_id),
            Capability::UsersWrite,
            AuditOutcome::Allowed,
            Some("mfa_disabled"),
            Some(user_id),
            None,
            None,
            None,
        )
        .await;

    tracing::info!(user_id = %user_id, "admin mfa disabled");
    Ok(Json(DisableResponse {
        message: "MFA has been disabled. All capability-gated actions will now be refused until you re-enroll.".into(),
    }))
}

fn internal() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "internal" })),
    )
}

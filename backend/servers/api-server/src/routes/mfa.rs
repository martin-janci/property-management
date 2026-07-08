//! Multi-Factor Authentication routes (Epic 9, Story 9.1 + 9.2).

use api_core::extractors::RlsConnection;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{AuditAction, CreateAuditLog, CreateTwoFactorAuth};
use serde::{Deserialize, Serialize};
use sqlx::Acquire;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use utoipa::ToSchema;

use crate::state::AppState;

// ── Generic per-user in-process brute-force throttle ────────────────────────
//
// Both the recovery-code verify (GH #1523/#1583) and the MFA setup-verify
// (issue #2159) endpoints are bearer-authenticated brute-force surfaces keyed
// on the authenticated `user_id`. They share one sliding-window limiter,
// differing only in their backing map + tuning constants. This mirrors the
// per-key in-process limiter in `routes/caddy_ask.rs`. NOTE: the counter is
// per-process; a Redis-backed counter (the sessions Redis is already in the
// stack) would hold the limit across instances — tracked as a follow-up.

struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

type UserRateLimiter = Mutex<HashMap<uuid::Uuid, RateLimitEntry>>;

/// Record an attempt for `user_id` in `limiter` and report whether it is within
/// the limit. Returns `false` once more than `max_attempts` attempts occur in a
/// rolling `window`.
///
/// Expired entries are evicted (GH #1583), but only once the map exceeds
/// `sweep_threshold`, so the `retain` walk is amortized rather than run on every
/// request. This matters because these maps are keyed on the whole user
/// population on auth paths: without eviction an attacker enumerating user_ids
/// could grow the map without bound (slow memory-exhaustion DoS).
fn rate_limit_allowed(
    limiter: &UserRateLimiter,
    user_id: uuid::Uuid,
    max_attempts: u32,
    window: Duration,
    sweep_threshold: usize,
) -> bool {
    let mut map = match limiter.lock() {
        Ok(m) => m,
        Err(poisoned) => poisoned.into_inner(),
    };
    let now = Instant::now();

    if map.len() > sweep_threshold {
        map.retain(|_, e| now.duration_since(e.window_start) < window);
    }

    let entry = map.entry(user_id).or_insert(RateLimitEntry {
        count: 0,
        window_start: now,
    });
    if now.duration_since(entry.window_start) >= window {
        entry.count = 0;
        entry.window_start = now;
    }
    entry.count += 1;
    entry.count <= max_attempts
}

// ── Brute-force throttle for recovery-code verification (GH #1523) ──────────
//
// `verify_recovery_code` is a full MFA-bypass surface on success, so repeated
// guesses must be throttled, keyed on `user_id` so one user's attempts can
// never lock out another. A successful verification clears the user's window.
const MFA_RECOVERY_MAX_ATTEMPTS: u32 = 10;
const MFA_RECOVERY_WINDOW: Duration = Duration::from_secs(900);
/// Only sweep expired limiter entries once the map exceeds this size (GH #1583).
const MFA_RECOVERY_SWEEP_THRESHOLD: usize = 1024;

static MFA_RECOVERY_RATE_LIMITER: LazyLock<UserRateLimiter> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Record an attempt for `user_id` and report whether it is within the limit.
/// Returns `false` once more than `MFA_RECOVERY_MAX_ATTEMPTS` attempts occur in
/// a rolling `MFA_RECOVERY_WINDOW`.
fn recovery_attempt_allowed(user_id: uuid::Uuid) -> bool {
    rate_limit_allowed(
        &MFA_RECOVERY_RATE_LIMITER,
        user_id,
        MFA_RECOVERY_MAX_ATTEMPTS,
        MFA_RECOVERY_WINDOW,
        MFA_RECOVERY_SWEEP_THRESHOLD,
    )
}

/// Clear a user's attempt counter after a successful verification, so a
/// legitimate user is never penalised by earlier mistyped codes. Also used by
/// tests to reset the process-global limiter for a given user (GH #1583).
pub fn recovery_attempts_reset(user_id: uuid::Uuid) {
    if let Ok(mut map) = MFA_RECOVERY_RATE_LIMITER.lock() {
        map.remove(&user_id);
    }
}

// ── Brute-force throttle for MFA setup-verification (issue #2159) ───────────
//
// `POST /api/v1/auth/mfa/verify` completes enrolment by checking a 6-digit TOTP
// against the pending secret. The login-path TOTP check is already throttled
// (`SessionRepository::check_rate_limit` — 429 after 5 email-keyed failures)
// and the recovery path above is throttled, but this endpoint had NO limiter,
// leaving a 10^6 code space brute-forceable by an authenticated caller who
// replays `/verify` with the pending secret still active. Same in-process
// sliding-window limiter as recovery, keyed on the authenticated `user_id`.
const MFA_SETUP_VERIFY_MAX_ATTEMPTS: u32 = 10;
const MFA_SETUP_VERIFY_WINDOW: Duration = Duration::from_secs(900);
const MFA_SETUP_VERIFY_SWEEP_THRESHOLD: usize = 1024;

static MFA_SETUP_VERIFY_RATE_LIMITER: LazyLock<UserRateLimiter> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Record a setup-verify attempt for `user_id`; `false` once more than
/// `MFA_SETUP_VERIFY_MAX_ATTEMPTS` occur in a rolling `MFA_SETUP_VERIFY_WINDOW`.
fn setup_verify_attempt_allowed(user_id: uuid::Uuid) -> bool {
    rate_limit_allowed(
        &MFA_SETUP_VERIFY_RATE_LIMITER,
        user_id,
        MFA_SETUP_VERIFY_MAX_ATTEMPTS,
        MFA_SETUP_VERIFY_WINDOW,
        MFA_SETUP_VERIFY_SWEEP_THRESHOLD,
    )
}

/// Clear a user's setup-verify counter (on successful enable, and for tests to
/// reset the process-global limiter).
pub fn setup_verify_attempts_reset(user_id: uuid::Uuid) {
    if let Ok(mut map) = MFA_SETUP_VERIFY_RATE_LIMITER.lock() {
        map.remove(&user_id);
    }
}

/// Create MFA router (mounted at `/api/v1/auth/mfa`).
///
/// The recovery-code verify endpoint is mounted separately at
/// `/api/v1/users/me/mfa/recovery-codes/verify` by the caller (see `lib.rs`).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", post(setup_mfa))
        .route("/verify", post(verify_mfa_setup))
        .route("/disable", post(disable_mfa))
        .route("/status", get(mfa_status))
        .route("/backup-codes/regenerate", post(regenerate_backup_codes))
}

/// Create the recovery-codes sub-router (mounted at
/// `/api/v1/users/me/mfa/recovery-codes`).
pub fn recovery_codes_router() -> Router<AppState> {
    Router::new().route("/verify", post(verify_recovery_code))
}

// ==================== Setup MFA (Story 9.1) ====================

/// MFA setup response.
///
/// Note: Recovery codes are issued on the `/verify` call (when MFA is
/// actually enabled), not here.  This keeps the setup step idempotent
/// (can be retried before verification without burning codes).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MfaSetupResponse {
    /// The TOTP secret (only shown once, for manual entry)
    pub secret: String,
    /// URI for QR code generation
    pub qr_uri: String,
}

/// Setup MFA endpoint.
/// Initiates 2FA setup by generating a secret and backup codes.
/// The user must verify with a code before 2FA is activated.
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/setup",
    tag = "Multi-Factor Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "MFA setup initiated", body = MfaSetupResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 409, description = "MFA already enabled", body = ErrorResponse)
    )
)]
pub async fn setup_mfa(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if MFA is already enabled
    if let Ok(Some(existing)) = state
        .two_factor_repo
        .get_by_user_id_rls(&mut **rls.conn(), user_id)
        .await
    {
        if existing.enabled {
            rls.release().await;
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "MFA_ALREADY_ENABLED",
                    "Two-factor authentication is already enabled. Disable it first to reconfigure.",
                )),
            ));
        }
    }

    // Generate new secret
    let secret = state.totp_service.generate_secret().map_err(|e| {
        tracing::error!(error = %e, "Failed to generate TOTP secret");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "SECRET_GENERATION_ERROR",
                "Failed to generate authentication secret",
            )),
        )
    })?;

    // Generate QR URI
    let qr_uri = state
        .totp_service
        .generate_qr_uri(&claims.email, &secret)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate QR URI");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "QR_GENERATION_ERROR",
                    "Failed to generate QR code",
                )),
            )
        })?;

    // Encrypt secret before storage - encryption is REQUIRED for security
    if !state.totp_service.is_encryption_enabled() {
        tracing::error!("TOTP encryption key not configured - MFA setup blocked for security");
        rls.release().await;
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "ENCRYPTION_NOT_CONFIGURED",
                "Two-factor authentication is temporarily unavailable. Please contact support.",
            )),
        ));
    }

    let stored_secret = state.totp_service.encrypt_secret(&secret).map_err(|e| {
        tracing::error!(error = %e, "Failed to encrypt TOTP secret");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_ERROR",
                "Failed to encrypt authentication secret",
            )),
        )
    })?;

    // Store the pending setup (not enabled yet). Recovery codes are generated
    // only on the /verify call (when MFA actually becomes active) so that
    // a retried /setup does not burn fresh codes prematurely.
    let create_data = CreateTwoFactorAuth {
        user_id,
        secret: stored_secret,
        backup_codes: vec![], // populated in mfa_recovery_codes on verify
    };

    state
        .two_factor_repo
        .create_rls(&mut **rls.conn(), create_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to store MFA setup");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to save authentication setup",
                )),
            )
        })?;

    tracing::info!(user_id = %user_id, "MFA setup initiated");

    rls.release().await;

    Ok(Json(MfaSetupResponse { secret, qr_uri }))
}

// ==================== Verify MFA Setup (Story 9.1) ====================

/// MFA verification request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyMfaRequest {
    /// The 6-digit TOTP code from authenticator app
    pub code: String,
}

/// MFA verification response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyMfaResponse {
    /// Success message
    pub message: String,
    /// Whether MFA is now enabled
    pub enabled: bool,
    /// 10 single-use recovery codes (shown ONCE — user must save them).
    /// Each code is usable via `POST /api/v1/users/me/mfa/recovery-codes/verify`.
    pub recovery_codes: Vec<String>,
}

/// Verify MFA setup endpoint.
/// Completes 2FA setup by verifying the user can generate valid codes.
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/verify",
    tag = "Multi-Factor Authentication",
    security(("bearer_auth" = [])),
    request_body = VerifyMfaRequest,
    responses(
        (status = 200, description = "MFA enabled", body = VerifyMfaResponse),
        (status = 400, description = "Invalid code", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "No MFA setup found", body = ErrorResponse)
    )
)]
pub async fn verify_mfa_setup(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Json(req): Json<VerifyMfaRequest>,
) -> Result<Json<VerifyMfaResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Brute-force throttle (issue #2159): a 6-digit TOTP has a 10^6 code space,
    // so cap verification attempts per user within a rolling window before doing
    // any work. The (N+1)th attempt in the window is rejected with 429; a
    // successful enable later clears the counter (see below). Runs before any DB
    // access so a flood cannot amplify into load.
    if !setup_verify_attempt_allowed(user_id) {
        tracing::warn!(%user_id, "mfa/verify: setup-verification rate limited");
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new(
                "RATE_LIMITED",
                "Too many verification attempts. Please try again later.",
            )),
        ));
    }

    // Get pending MFA setup
    let mfa_record = state
        .two_factor_repo
        .get_by_user_id_rls(&mut **rls.conn(), user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch MFA record");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch MFA setup",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MFA_NOT_SETUP",
                    "No MFA setup found. Please initiate setup first.",
                )),
            )
        })?;

    if mfa_record.enabled {
        rls.release().await;
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "MFA_ALREADY_ENABLED",
                "Two-factor authentication is already enabled.",
            )),
        ));
    }

    // Decrypt secret if needed
    let decrypted_secret = state
        .totp_service
        .decrypt_secret(&mfa_record.secret)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to decrypt TOTP secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DECRYPTION_ERROR",
                    "Failed to decrypt authentication secret",
                )),
            )
        })?;

    // Verify the TOTP code
    let is_valid = state
        .totp_service
        .verify_code(&decrypted_secret, &req.code)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify TOTP code");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "VERIFICATION_ERROR",
                    "Failed to verify code",
                )),
            )
        })?;

    if !is_valid {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CODE",
                "The verification code is invalid. Please check your authenticator app and try again.",
            )),
        ));
    }

    // Generate 10 single-use recovery codes.
    let (plain_codes, hashed_codes) = state.totp_service.generate_backup_codes().map_err(|e| {
        tracing::error!(error = %e, "Failed to generate recovery codes");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "RECOVERY_CODES_ERROR",
                "Failed to generate recovery codes",
            )),
        )
    })?;

    // Enable MFA and issue recovery codes in a single transaction so a crash
    // between the two steps cannot leave MFA enabled with zero recovery codes.
    //
    // The transaction runs on the SAME RLS connection used above (not a second
    // `state.db.begin()` raw-pool connection), so `app.current_user_id` is set
    // and the self-policies on `user_2fa` / `mfa_recovery_codes` (migrations
    // 00024 / 00149) enforce — the `WHERE user_id = $1` clause scoped to the
    // verified JWT subject is then defense-in-depth rather than the sole
    // isolation, and the handler stays on one connection / one RLS context.
    let mut tx = (&mut **rls.conn()).begin().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to begin recovery codes transaction");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to issue recovery codes",
            )),
        )
    })?;

    // 1. Enable MFA — inside the transaction so it rolls back if code insertion fails.
    state
        .two_factor_repo
        .enable_rls(&mut *tx, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to enable MFA in transaction");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to enable MFA")),
            )
        })?;

    // 2. Remove any stale codes from a previous enable/disable cycle.
    sqlx::query("DELETE FROM mfa_recovery_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to clear old recovery codes");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to issue recovery codes",
                )),
            )
        })?;

    for hash in &hashed_codes {
        sqlx::query("INSERT INTO mfa_recovery_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to insert recovery code");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to issue recovery codes",
                    )),
                )
            })?;
    }

    // Keep user_2fa.backup_codes_remaining in sync so /status reflects the count.
    sqlx::query("UPDATE user_2fa SET backup_codes_remaining = $1 WHERE user_id = $2")
        .bind(hashed_codes.len() as i32)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to sync backup_codes_remaining");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to issue recovery codes",
                )),
            )
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to commit recovery codes transaction");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to issue recovery codes",
            )),
        )
    })?;

    // Log MFA enabled (Story 9.6 - Audit logging)
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::MfaEnabled,
            resource_type: Some("two_factor_auth".to_string()),
            resource_id: Some(user_id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user_id, "Failed to create audit log for MFA enabled");
    }

    tracing::info!(user_id = %user_id, codes = plain_codes.len(), "MFA enabled successfully; recovery codes issued");

    // Clear the brute-force counter now that enrolment succeeded (issue #2159),
    // so earlier mistyped codes don't leave a lingering window for this user.
    setup_verify_attempts_reset(user_id);

    rls.release().await;

    Ok(Json(VerifyMfaResponse {
        message: "Two-factor authentication has been enabled. Store your recovery codes — they will not be shown again.".to_string(),
        enabled: true,
        recovery_codes: plain_codes,
    }))
}

// ==================== Disable MFA (Story 9.2) ====================

/// Disable MFA request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DisableMfaRequest {
    /// Current TOTP code or backup code to confirm
    pub code: String,
}

/// Disable MFA response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisableMfaResponse {
    /// Success message
    pub message: String,
}

/// Disable MFA endpoint.
/// Requires verification code to prevent unauthorized disabling.
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/disable",
    tag = "Multi-Factor Authentication",
    security(("bearer_auth" = [])),
    request_body = DisableMfaRequest,
    responses(
        (status = 200, description = "MFA disabled", body = DisableMfaResponse),
        (status = 400, description = "Invalid code", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "MFA not enabled", body = ErrorResponse)
    )
)]
pub async fn disable_mfa(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Json(req): Json<DisableMfaRequest>,
) -> Result<Json<DisableMfaResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Get MFA record
    let mfa_record = state
        .two_factor_repo
        .get_by_user_id_rls(&mut **rls.conn(), user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch MFA record");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch MFA status",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MFA_NOT_ENABLED",
                    "Two-factor authentication is not enabled",
                )),
            )
        })?;

    if !mfa_record.enabled {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "MFA_NOT_ENABLED",
                "Two-factor authentication is not enabled",
            )),
        ));
    }

    // Decrypt secret if encrypted
    let decrypted_secret = state
        .totp_service
        .decrypt_secret(&mfa_record.secret)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to decrypt TOTP secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DECRYPTION_ERROR",
                    "Failed to verify code",
                )),
            )
        })?;

    // Try to verify as TOTP code first
    let is_totp_valid = state
        .totp_service
        .verify_code(&decrypted_secret, &req.code)
        .unwrap_or(false);

    // If TOTP failed, try recovery codes from the mfa_recovery_codes table.
    // The JSONB backup_codes field in user_2fa is no longer used for new enrollments;
    // new codes are stored in mfa_recovery_codes (per-row, single-use).
    let is_valid = if is_totp_valid {
        true
    } else {
        // Load unused recovery codes for this user.
        let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
            "SELECT id, code_hash FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .fetch_all(&mut **rls.conn())
        .await
        .unwrap_or_default();

        if rows.is_empty() {
            // Fall back to legacy JSONB backup_codes for users enrolled before the migration.
            let backup_codes: Vec<String> =
                serde_json::from_value(mfa_record.backup_codes.clone()).unwrap_or_default();
            state
                .totp_service
                .verify_backup_code(&req.code, &backup_codes)
                .map(|result| result.is_some())
                .unwrap_or(false)
        } else {
            let normalized = req.code.trim().replace(['-', ' '], "").to_uppercase();
            let hashes: Vec<String> = rows.iter().map(|(_, h)| h.clone()).collect();
            state
                .totp_service
                .verify_backup_code(&normalized, &hashes)
                .map(|result| result.is_some())
                .unwrap_or(false)
        }
    };

    if !is_valid {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CODE",
                "Invalid verification code. Use your authenticator app or a recovery code.",
            )),
        ));
    }

    // Disable MFA and invalidate all recovery codes atomically, on the SAME
    // RLS connection used above — not a second `state.db.begin()` raw-pool
    // connection. Keeping the whole auth-critical handler on one connection
    // means one RLS context: `app.current_user_id` is set, so the self-policies
    // on `mfa_recovery_codes` / `user_2fa` (migrations 00149 / 00024) enforce,
    // and the `WHERE user_id = $1` literal is defense-in-depth rather than the
    // sole isolation (same rationale as verify_mfa_setup above). Both writes
    // share one transaction so a crash between them cannot leave MFA enabled
    // with live recovery codes (or recovery codes wiped with MFA still on).
    let mut tx = (&mut **rls.conn()).begin().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to begin disable transaction");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to disable MFA",
            )),
        )
    })?;

    // Mark all recovery codes as used (invalidate without deleting for audit trail).
    sqlx::query(
        "UPDATE mfa_recovery_codes SET used_at = NOW() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to invalidate recovery codes");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to disable MFA",
            )),
        )
    })?;

    // Disable in user_2fa within the SAME transaction.
    state
        .two_factor_repo
        .disable_rls(&mut *tx, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to disable MFA");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to disable MFA",
                )),
            )
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to commit disable transaction");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to disable MFA",
            )),
        )
    })?;

    // Log MFA disabled (Story 9.6 - Audit logging)
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::MfaDisabled,
            resource_type: Some("two_factor_auth".to_string()),
            resource_id: Some(user_id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user_id, "Failed to create audit log for MFA disabled");
    }

    tracing::info!(user_id = %user_id, "MFA disabled; recovery codes invalidated");

    rls.release().await;

    Ok(Json(DisableMfaResponse {
        message: "Two-factor authentication has been disabled".to_string(),
    }))
}

// ==================== MFA Status (Story 9.1) ====================

/// MFA status response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MfaStatusResponse {
    /// Whether MFA is enabled
    pub enabled: bool,
    /// When MFA was enabled (if enabled)
    pub enabled_at: Option<String>,
    /// Remaining backup codes count
    pub backup_codes_remaining: i32,
}

/// Get MFA status endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/auth/mfa/status",
    tag = "Multi-Factor Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "MFA status", body = MfaStatusResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn mfa_status(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
) -> Result<Json<MfaStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Get MFA record
    let mfa_record = state
        .two_factor_repo
        .get_by_user_id_rls(&mut **rls.conn(), user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch MFA status");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch MFA status",
                )),
            )
        })?;

    rls.release().await;

    match mfa_record {
        Some(record) if record.enabled => Ok(Json(MfaStatusResponse {
            enabled: true,
            enabled_at: record.enabled_at.map(|dt| dt.to_rfc3339()),
            backup_codes_remaining: record.backup_codes_remaining,
        })),
        _ => Ok(Json(MfaStatusResponse {
            enabled: false,
            enabled_at: None,
            backup_codes_remaining: 0,
        })),
    }
}

// ==================== Regenerate Backup Codes (Story 9.2) ====================

/// Regenerate backup codes request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegenerateBackupCodesRequest {
    /// Current TOTP code to authorize regeneration
    pub code: String,
}

/// Regenerate backup codes response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateBackupCodesResponse {
    /// New backup codes (only shown once)
    pub backup_codes: Vec<String>,
    /// Success message
    pub message: String,
}

/// Regenerate backup codes endpoint.
/// Generates new backup codes, invalidating all previous ones.
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/backup-codes/regenerate",
    tag = "Multi-Factor Authentication",
    security(("bearer_auth" = [])),
    request_body = RegenerateBackupCodesRequest,
    responses(
        (status = 200, description = "Backup codes regenerated", body = RegenerateBackupCodesResponse),
        (status = 400, description = "Invalid code", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "MFA not enabled", body = ErrorResponse)
    )
)]
pub async fn regenerate_backup_codes(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Json(req): Json<RegenerateBackupCodesRequest>,
) -> Result<Json<RegenerateBackupCodesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Get MFA record
    let mfa_record = state
        .two_factor_repo
        .get_by_user_id_rls(&mut **rls.conn(), user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch MFA record");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch MFA status",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MFA_NOT_ENABLED",
                    "Two-factor authentication is not enabled",
                )),
            )
        })?;

    if !mfa_record.enabled {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "MFA_NOT_ENABLED",
                "Two-factor authentication is not enabled",
            )),
        ));
    }

    // Decrypt secret if encrypted
    let decrypted_secret = state
        .totp_service
        .decrypt_secret(&mfa_record.secret)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to decrypt TOTP secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DECRYPTION_ERROR",
                    "Failed to verify code",
                )),
            )
        })?;

    // Verify TOTP code
    let is_valid = state
        .totp_service
        .verify_code(&decrypted_secret, &req.code)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify TOTP code");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "VERIFICATION_ERROR",
                    "Failed to verify code",
                )),
            )
        })?;

    if !is_valid {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CODE",
                "Invalid verification code",
            )),
        ));
    }

    // Generate new backup codes
    let (plain_codes, hashed_codes) = state.totp_service.generate_backup_codes().map_err(|e| {
        tracing::error!(error = %e, "Failed to generate backup codes");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "BACKUP_CODES_ERROR",
                "Failed to generate backup codes",
            )),
        )
    })?;

    // Update the backup codes
    state
        .two_factor_repo
        .regenerate_backup_codes_rls(&mut **rls.conn(), user_id, hashed_codes)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save new backup codes");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to save backup codes",
                )),
            )
        })?;

    // Log backup codes regenerated (Story 9.6 - Audit logging)
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::MfaBackupCodesRegenerated,
            resource_type: Some("two_factor_auth".to_string()),
            resource_id: Some(user_id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user_id, "Failed to create audit log for backup codes regenerated");
    }

    tracing::info!(user_id = %user_id, "Backup codes regenerated");

    rls.release().await;

    Ok(Json(RegenerateBackupCodesResponse {
        backup_codes: plain_codes,
        message: "Backup codes have been regenerated. Save them securely.".to_string(),
    }))
}

// ==================== Verify Recovery Code (Story 9.2) ====================

/// Request body for `POST /api/v1/users/me/mfa/recovery-codes/verify`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyRecoveryCodeRequest {
    /// Plaintext recovery code (as shown at MFA enable time).
    pub code: String,
}

/// Response for `POST /api/v1/users/me/mfa/recovery-codes/verify`.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRecoveryCodeResponse {
    /// Success message.
    pub message: String,
    /// How many recovery codes remain after this call.
    pub codes_remaining: i64,
}

/// Consume a single-use MFA recovery code.
///
/// The code is matched (via Argon2 hash comparison) against the unused rows in
/// `mfa_recovery_codes` that belong to the authenticated user.  On success:
///   1. The matched row is marked `used_at = NOW()` (single-use guarantee).
///   2. An audit log entry is emitted.
///
/// Codes are invalidated atomically via a `used_at IS NULL` guard + row-affected
/// check to prevent replay attacks under concurrent requests.
#[utoipa::path(
    post,
    path = "/api/v1/users/me/mfa/recovery-codes/verify",
    tag = "Multi-Factor Authentication",
    security(("bearer_auth" = [])),
    request_body = VerifyRecoveryCodeRequest,
    responses(
        (status = 200, description = "Recovery code accepted", body = VerifyRecoveryCodeResponse),
        (status = 400, description = "Invalid or already-used code", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "MFA not enabled", body = ErrorResponse),
        (status = 400, description = "Invalid or exhausted recovery code", body = ErrorResponse)
    )
)]
pub async fn verify_recovery_code(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Json(req): Json<VerifyRecoveryCodeRequest>,
) -> Result<Json<VerifyRecoveryCodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Brute-force throttle (GH #1523): this endpoint is a full MFA bypass on
    // success, so cap attempts per user within a rolling window before doing any
    // work. The (N+1)th attempt in the window is rejected with 429; a successful
    // verification later clears the counter.
    if !recovery_attempt_allowed(user_id) {
        if let Err(e) = state
            .audit_log_repo
            .create(CreateAuditLog {
                user_id: Some(user_id),
                action: AuditAction::MfaRecoveryRateLimited,
                resource_type: Some("mfa_recovery_rate_limited".to_string()),
                resource_id: Some(user_id),
                org_id: None,
                details: Some(serde_json::json!({ "outcome": "denied", "reason": "rate_limited" })),
                old_values: None,
                new_values: None,
                ip_address: None,
                user_agent: None,
            })
            .await
        {
            tracing::error!(error = %e, "Failed to write audit log for rate-limited recovery verify");
        }
        rls.release().await;
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new(
                "RATE_LIMITED",
                "Too many recovery-code attempts. Please try again later.",
            )),
        ));
    }

    // Run every query on the RLS connection so `app.current_user_id` is set for
    // the duration of the handler. The self-policies on `user_2fa` /
    // `mfa_recovery_codes` (migrations 00024 / 00149) gate on
    // `app.current_user_id`, so this is what makes them enforce — the
    // `WHERE user_id = $1` literal is then defense-in-depth, not the sole
    // isolation. (Previously this acquired `acquire_public()`, which *clears*
    // the GUC — leaving 100% of the isolation on the literal and silently
    // turning into deny-all the moment these tables get FORCE RLS.)
    // `rls.release()` is called before every return so the context is cleared
    // before the connection returns to the pool.

    // Verify MFA is enabled for this user.
    let enrolled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&mut **rls.conn())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "recovery/verify: fetch enrollment failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to check MFA status",
                    )),
                )
            })?;

    if !matches!(enrolled, Some(true)) {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "MFA_NOT_ENABLED",
                "Two-factor authentication is not enabled.",
            )),
        ));
    }

    // Load unused recovery codes for this user.
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, code_hash FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_all(&mut **rls.conn())
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "recovery/verify: fetch codes failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to fetch recovery codes",
            )),
        )
    })?;

    if rows.is_empty() {
        if let Err(e) = state
            .audit_log_repo
            .create(CreateAuditLog {
                user_id: Some(user_id),
                action: AuditAction::MfaBackupCodeUsed,
                resource_type: Some("mfa_recovery_no_codes".to_string()),
                resource_id: Some(user_id),
                org_id: None,
                details: Some(
                    serde_json::json!({ "outcome": "denied", "reason": "no_codes_remaining" }),
                ),
                old_values: None,
                new_values: None,
                ip_address: None,
                user_agent: None,
            })
            .await
        {
            tracing::error!(error = %e, "Failed to write audit log for exhausted recovery codes");
        }
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_RECOVERY_CODE",
                "Recovery code did not match. Check the code and try again.",
            )),
        ));
    }

    // Constant-time Argon2 comparison against each unused hash.
    let normalized = req.code.trim().replace(['-', ' '], "").to_uppercase();
    let hashes: Vec<String> = rows.iter().map(|(_, h)| h.clone()).collect();
    let matched_idx = state
        .totp_service
        .verify_backup_code(&normalized, &hashes)
        .map_err(|e| {
            tracing::error!(error = %e, "recovery/verify: hash comparison failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "VERIFICATION_ERROR",
                    "Failed to verify recovery code",
                )),
            )
        })?;

    let Some(idx) = matched_idx else {
        if let Err(e) = state
            .audit_log_repo
            .create(CreateAuditLog {
                user_id: Some(user_id),
                action: AuditAction::MfaBackupCodeUsed,
                resource_type: Some("mfa_recovery_invalid".to_string()),
                resource_id: Some(user_id),
                org_id: None,
                details: Some(serde_json::json!({ "outcome": "denied" })),
                old_values: None,
                new_values: None,
                ip_address: None,
                user_agent: None,
            })
            .await
        {
            tracing::error!(error = %e, "Failed to write audit log for invalid recovery code");
        }
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_RECOVERY_CODE",
                "Recovery code did not match. Check the code and try again.",
            )),
        ));
    };

    let matched_id = rows[idx].0;

    // Mark as used atomically. The `AND used_at IS NULL` guard + rows_affected
    // check prevents replay under concurrent requests.
    let upd = sqlx::query(
        "UPDATE mfa_recovery_codes SET used_at = NOW() WHERE id = $1 AND used_at IS NULL",
    )
    .bind(matched_id)
    .execute(&mut **rls.conn())
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "recovery/verify: mark used failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to consume recovery code",
            )),
        )
    })?;

    if upd.rows_affected() == 0 {
        // Lost race to a concurrent caller — fail closed.
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_RECOVERY_CODE",
                "Recovery code did not match. Check the code and try again.",
            )),
        ));
    }

    // Count remaining unused codes.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(&mut **rls.conn())
    .await
    .unwrap_or(0);

    // Audit success.
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::MfaBackupCodeUsed,
            resource_type: Some("mfa_recovery_used".to_string()),
            resource_id: Some(user_id),
            org_id: None,
            details: Some(
                serde_json::json!({ "outcome": "allowed", "codes_remaining": remaining }),
            ),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, "Failed to write audit log for recovery code used");
    }

    tracing::info!(user_id = %user_id, remaining, "MFA recovery code consumed");

    // Successful verification clears the brute-force counter (GH #1523).
    recovery_attempts_reset(user_id);

    rls.release().await;

    Ok(Json(VerifyRecoveryCodeResponse {
        message: "Recovery code accepted.".to_string(),
        codes_remaining: remaining,
    }))
}

// ==================== Helper Functions ====================

/// Extract bearer token from Authorization header.
fn extract_bearer_token(
    headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    Ok(auth_header[7..].to_string())
}

/// Validate access token and return claims.
fn validate_access_token(
    state: &AppState,
    token: &str,
) -> Result<crate::services::jwt::Claims, (StatusCode, Json<ErrorResponse>)> {
    state.jwt_service.validate_access_token(token).map_err(|e| {
        tracing::debug!(error = %e, "Invalid access token");
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_TOKEN",
                "Invalid or expired token",
            )),
        )
    })
}

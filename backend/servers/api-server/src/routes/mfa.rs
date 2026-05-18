//! Multi-Factor Authentication routes (Epic 9, Story 9.1).

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
use utoipa::ToSchema;

use crate::state::AppState;

/// Create MFA router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", post(setup_mfa))
        .route("/verify", post(verify_mfa_setup))
        .route("/disable", post(disable_mfa))
        .route("/status", get(mfa_status))
        .route("/backup-codes/regenerate", post(regenerate_backup_codes))
}

// ==================== Setup MFA (Story 9.1) ====================

/// MFA setup response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MfaSetupResponse {
    /// The TOTP secret (only shown once, for manual entry)
    pub secret: String,
    /// URI for QR code generation
    pub qr_uri: String,
    /// Backup codes (only shown once, user must save them)
    pub backup_codes: Vec<String>,
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

    // Generate backup codes
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

    // Store the setup (not enabled yet)
    let create_data = CreateTwoFactorAuth {
        user_id,
        secret: stored_secret,
        backup_codes: hashed_codes,
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

    Ok(Json(MfaSetupResponse {
        secret,
        qr_uri,
        backup_codes: plain_codes,
    }))
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
        return Ok(Json(VerifyMfaResponse {
            message: "Two-factor authentication is already enabled".to_string(),
            enabled: true,
        }));
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

    // Enable MFA
    state
        .two_factor_repo
        .enable_rls(&mut **rls.conn(), user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to enable MFA");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to enable MFA")),
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

    tracing::info!(user_id = %user_id, "MFA enabled successfully");

    rls.release().await;

    Ok(Json(VerifyMfaResponse {
        message: "Two-factor authentication has been enabled".to_string(),
        enabled: true,
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

    // If TOTP failed, try backup codes
    let is_valid = if is_totp_valid {
        true
    } else {
        let backup_codes: Vec<String> =
            serde_json::from_value(mfa_record.backup_codes.clone()).unwrap_or_default();
        state
            .totp_service
            .verify_backup_code(&req.code, &backup_codes)
            .map(|result| result.is_some())
            .unwrap_or(false)
    };

    if !is_valid {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CODE",
                "Invalid verification code. Use your authenticator app or a backup code.",
            )),
        ));
    }

    // Disable MFA
    state
        .two_factor_repo
        .disable_rls(&mut **rls.conn(), user_id)
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

    tracing::info!(user_id = %user_id, "MFA disabled");

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

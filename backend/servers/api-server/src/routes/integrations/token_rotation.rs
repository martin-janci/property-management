//! Airbnb OAuth access-token rotation (Gap 83-1).
//!
//! Provides:
//!
//! * [`with_token_refresh`] — wraps any Airbnb API call: decrypts the stored
//!   access token, proactively refreshes it when it is within the refresh-buffer
//!   window, and retries once on `401 Unauthorized` from the Airbnb API.
//!
//! * Revocation endpoint: `POST /organizations/{org_id}/airbnb/token/revoke` —
//!   clears stored tokens locally (Airbnb's Partner API exposes no remote
//!   token-revocation endpoint, so revocation is local-only).
//!
//! # Refresh algorithm
//!
//! 1. Load the `RentalPlatformConnection` from the DB.
//! 2. Decrypt `canonical_encrypted_token()` (prefers `encrypted_token`, falls
//!    back to `access_token`).
//! 3. If `token_expires_at - now < REFRESH_BUFFER_SECS`, proactively refresh
//!    before the call.
//! 4. Execute the caller's closure with the (possibly refreshed) plaintext token.
//! 5. If the closure signals `TokenExpired` (HTTP 401 from Airbnb), refresh
//!    once more and retry the closure.
//! 6. On a successful refresh, persist the new tokens to both the canonical and
//!    legacy columns (via `update_airbnb_tokens`).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use db::models::rental::RentalPlatformConnection;
use integrations::{
    decrypt_if_available, encrypt_optional_required, encrypt_required, AirbnbClient, AirbnbError,
    AirbnbOAuthConfig, IntegrationCrypto,
};
use uuid::Uuid;

use super::sync::{verify_org_access, OrgIdPath};
use crate::state::AppState;
use common::errors::ErrorResponse;

/// Proactive refresh: refresh when token expires within this many seconds.
const REFRESH_BUFFER_SECS: i64 = 300; // 5 minutes

// ==================== Router ====================

/// Token-rotation sub-router.
pub fn router() -> Router<AppState> {
    Router::new().route(
        "/organizations/{org_id}/airbnb/token/revoke",
        post(revoke_airbnb_token),
    )
}

// ==================== Token Refresh Helper ====================

/// Outcome of [`with_token_refresh`].
#[derive(Debug)]
pub enum TokenRotationOutcome<T> {
    /// Call succeeded; returns the result.
    Ok(T),
    /// No Airbnb connection found for the org.
    NoConnection,
    /// Token is expired and there is no refresh token available.
    ExpiredNoRefresh,
    /// Decryption failed — crypto key missing or token corrupted.
    DecryptionFailed(String),
    /// Refresh call failed.
    RefreshFailed(String),
    /// Caller's closure failed (after refresh and retry).
    CallFailed(AirbnbError),
}

/// Execute `call` with the organisation's current decrypted Airbnb access token,
/// refreshing proactively or on 401 as needed.
///
/// # Type parameters
/// * `F` — async closure `(access_token: String) -> Result<T, AirbnbError>`
/// * `T` — return value on success
///
/// # Steps
/// 1. Load the connection from DB.
/// 2. Decrypt the stored encrypted token.
/// 3. Proactively refresh if within [`REFRESH_BUFFER_SECS`] of expiry.
/// 4. Call `call(access_token)`.
/// 5. On `AirbnbError::TokenExpired`, refresh once and retry.
/// 6. Persist any new tokens.
pub async fn with_token_refresh<F, Fut, T>(
    state: &AppState,
    org_id: Uuid,
    call: F,
) -> TokenRotationOutcome<T>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<T, AirbnbError>>,
{
    // 1. Load connection.
    let connection = match state
        .rental_repo
        .find_airbnb_connection_by_org(org_id)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => return TokenRotationOutcome::NoConnection,
        Err(e) => {
            tracing::error!(org_id = %org_id, error = %e, "DB error loading Airbnb connection");
            return TokenRotationOutcome::CallFailed(AirbnbError::Api(e.to_string()));
        }
    };

    // 2. Decrypt access token.
    let crypto = IntegrationCrypto::try_from_env();
    let enc_token = match connection.canonical_encrypted_token() {
        Some(t) => t.to_string(),
        None => {
            return TokenRotationOutcome::NoConnection;
        }
    };
    let mut plaintext_token = decrypt_if_available(crypto.as_ref(), &enc_token);
    if plaintext_token.starts_with('[') {
        // Decryption returned a placeholder — crypto key is missing or token corrupt.
        return TokenRotationOutcome::DecryptionFailed(plaintext_token);
    }

    // Build Airbnb client from cached AppState config.
    let oauth_config = AirbnbOAuthConfig {
        client_id: state.airbnb_config.client_id.clone(),
        client_secret: state.airbnb_config.client_secret.clone(),
        redirect_uri: state.airbnb_config.redirect_uri.clone(),
    };
    let client = AirbnbClient::new(oauth_config);

    // 3. Proactive refresh: refresh if token expires within buffer window.
    let needs_proactive_refresh = connection
        .token_expires_at
        .map(|exp| {
            let threshold = chrono::Utc::now() + chrono::Duration::seconds(REFRESH_BUFFER_SECS);
            exp <= threshold
        })
        .unwrap_or(false);

    if needs_proactive_refresh {
        tracing::info!(
            org_id = %org_id,
            connection_id = %connection.id,
            expires_at = ?connection.token_expires_at,
            "Airbnb token near expiry — proactively refreshing"
        );

        match attempt_refresh(&client, &connection, crypto.as_ref(), state, &connection.id).await {
            Ok(new_token) => {
                plaintext_token = new_token;
            }
            Err(e) => {
                tracing::warn!(
                    org_id = %org_id,
                    error = %e,
                    "Proactive Airbnb token refresh failed; attempting call with current token"
                );
                // Fall through — if the current token is still valid the call
                // will succeed; if not, the 401-retry path handles it.
            }
        }
    }

    // 4. Execute caller's closure.
    match call(plaintext_token.clone()).await {
        Ok(result) => TokenRotationOutcome::Ok(result),
        Err(AirbnbError::TokenExpired) => {
            // 5. Retry once after refresh.
            tracing::info!(
                org_id = %org_id,
                connection_id = %connection.id,
                "Airbnb API returned 401 — refreshing token and retrying"
            );
            match attempt_refresh(&client, &connection, crypto.as_ref(), state, &connection.id)
                .await
            {
                Ok(new_token) => match call(new_token).await {
                    Ok(result) => TokenRotationOutcome::Ok(result),
                    Err(e) => TokenRotationOutcome::CallFailed(e),
                },
                Err(e) => TokenRotationOutcome::RefreshFailed(e),
            }
        }
        Err(e) => TokenRotationOutcome::CallFailed(e),
    }
}

/// Perform a token refresh using the stored encrypted refresh token.
///
/// On success, persists the new tokens to the DB and returns the new plaintext
/// access token.
async fn attempt_refresh(
    client: &AirbnbClient,
    connection: &RentalPlatformConnection,
    crypto: Option<&IntegrationCrypto>,
    state: &AppState,
    connection_id: &Uuid,
) -> Result<String, String> {
    // Decrypt the refresh token.
    let enc_refresh = match connection.canonical_encrypted_refresh_token() {
        Some(r) => r.to_string(),
        None => {
            return Err("no refresh token stored for this connection".to_string());
        }
    };

    let crypto_ref = crypto;
    let plaintext_refresh = decrypt_if_available(crypto_ref, &enc_refresh);
    if plaintext_refresh.starts_with('[') {
        return Err(format!(
            "refresh token decryption failed: {}",
            plaintext_refresh
        ));
    }

    // Call Airbnb token endpoint.
    let new_tokens = client.refresh_token(&plaintext_refresh).await.map_err(|e| {
        tracing::error!(connection_id = %connection_id, error = %e, "Airbnb token refresh failed");
        e.to_string()
    })?;

    // Encrypt and persist.
    let enc_access = encrypt_required(crypto_ref, &new_tokens.access_token).map_err(|e| {
        tracing::error!(connection_id = %connection_id, error = %e, "Failed to encrypt refreshed access token");
        e.to_string()
    })?;

    let enc_refresh_new =
        encrypt_optional_required(crypto_ref, new_tokens.refresh_token.as_deref()).map_err(
            |e| {
                tracing::error!(connection_id = %connection_id, error = %e, "Failed to encrypt refreshed refresh token");
                e.to_string()
            },
        )?;

    state
        .rental_repo
        .update_airbnb_tokens(
            *connection_id,
            &enc_access,
            enc_refresh_new.as_deref(),
            new_tokens.expires_at,
        )
        .await
        .map_err(|e| {
            tracing::error!(connection_id = %connection_id, error = %e, "Failed to persist rotated Airbnb tokens");
            e.to_string()
        })?;

    tracing::info!(
        connection_id = %connection_id,
        expires_at = ?new_tokens.expires_at,
        token_rotated = new_tokens.refresh_token.is_some(),
        "Airbnb token rotated and persisted"
    );

    Ok(new_tokens.access_token)
}

// ==================== Revocation Endpoint ====================

/// Airbnb token revocation response.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct AirbnbTokenRevokeResponse {
    pub success: bool,
    pub message: String,
    /// Whether the remote Airbnb revocation call succeeded.
    /// `false` means only the local record was cleared.
    pub remote_revoked: bool,
}

/// Revoke Airbnb OAuth tokens for an organisation.
///
/// Clears the stored access/refresh tokens in the database. Airbnb's Partner
/// API exposes no remote token-revocation endpoint, so this is local-only; the
/// access token expires naturally within its TTL. `remote_revoked` is therefore
/// always `false`.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/token/revoke",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Airbnb tokens revoked", body = AirbnbTokenRevokeResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Airbnb connection found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn revoke_airbnb_token(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<AirbnbTokenRevokeResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Revoking Airbnb tokens"
    );

    // IDOR guard.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let rental_repo = &state.rental_repo;

    // Load connection — return 404 if none exists.
    let connection = rental_repo
        .find_airbnb_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find Airbnb connection for revocation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to look up Airbnb connection",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "No Airbnb connection found for this organisation",
                )),
            )
        })?;

    // Airbnb's Partner API does not expose a token-revocation endpoint, so
    // revocation is local-only: we clear the stored tokens below and the
    // access token expires naturally within its TTL. `remote_revoked` is
    // therefore always false; we keep the field for forward-compatibility.
    let crypto = IntegrationCrypto::try_from_env();
    let remote_revoked = if let Some(enc_token) = connection.canonical_encrypted_token() {
        let plaintext = decrypt_if_available(crypto.as_ref(), enc_token);
        if !plaintext.starts_with('[') {
            tracing::info!(
                connection_id = %connection.id,
                "Airbnb token revocation: local-only (no remote revoke API)"
            );
            false
        } else {
            tracing::warn!(
                connection_id = %connection.id,
                "Cannot revoke remotely: token decryption failed"
            );
            false
        }
    } else {
        false
    };

    // Clear tokens locally — this is the authoritative step.
    let revoked_count = rental_repo
        .revoke_airbnb_connection(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to revoke Airbnb connection locally");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to clear Airbnb tokens",
                )),
            )
        })?;

    tracing::info!(
        connection_id = %connection.id,
        org_id = %path.org_id,
        revoked_count = revoked_count,
        remote_revoked = remote_revoked,
        "Airbnb token revocation completed"
    );

    Ok(Json(AirbnbTokenRevokeResponse {
        success: true,
        message: "Airbnb tokens revoked successfully".to_string(),
        remote_revoked,
    }))
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_buffer_secs_is_positive() {
        const {
            assert!(REFRESH_BUFFER_SECS > 0);
            // Must be at least the minimum enforced by TokenRefreshConfig.
            assert!(REFRESH_BUFFER_SECS >= integrations::MIN_REFRESH_BUFFER_SECS);
        }
    }

    #[test]
    fn test_token_rotation_outcome_variants() {
        // Ensure the enum variants are accessible (compile-time check).
        let _: TokenRotationOutcome<String> = TokenRotationOutcome::NoConnection;
        let _: TokenRotationOutcome<String> = TokenRotationOutcome::ExpiredNoRefresh;
        let _: TokenRotationOutcome<String> =
            TokenRotationOutcome::DecryptionFailed("test".to_string());
        let _: TokenRotationOutcome<String> =
            TokenRotationOutcome::RefreshFailed("test".to_string());
    }

    #[test]
    fn test_proactive_refresh_threshold() {
        let now = chrono::Utc::now();

        // Token expiring in 2 minutes — within the 5-minute buffer.
        let exp_soon = now + chrono::Duration::seconds(120);
        let threshold = now + chrono::Duration::seconds(REFRESH_BUFFER_SECS);
        assert!(
            exp_soon <= threshold,
            "Token expiring in 2 min should need proactive refresh"
        );

        // Token expiring in 10 minutes — outside the buffer.
        let exp_later = now + chrono::Duration::seconds(600);
        assert!(
            exp_later > threshold,
            "Token expiring in 10 min should NOT need proactive refresh"
        );
    }

    #[test]
    fn test_airbnb_token_revoke_response_structure() {
        let resp = AirbnbTokenRevokeResponse {
            success: true,
            message: "Revoked".to_string(),
            remote_revoked: false,
        };
        assert!(resp.success);
        assert!(!resp.remote_revoked);
    }
}

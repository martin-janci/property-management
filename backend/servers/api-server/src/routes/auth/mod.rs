//! Authentication routes (UC-14, Epic 1).

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::errors::{ErrorResponse, ValidationError};
use db::models::{AuditAction, CreateAuditLog, CreateUser, Locale, UpdateUser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use utoipa::ToSchema;

use crate::routes::rate_limit::{rate_limit_allowed, InProcessRateLimiter};
use crate::services::AuthService;
use crate::state::AppState;

// ── Email-keyed throttle for unauthenticated email-dispatch surfaces ────────
//
// `POST /api/v1/auth/forgot-password` and `POST /api/v1/auth/resend-verification`
// each send an email and rotate the recipient's outstanding token on every
// call, with no authentication. Without a limiter an attacker can (a) mailbomb
// a victim's inbox and (b) repeatedly clobber a pending reset/verification
// token so a legitimate link the victim is mid-flow on stops working. Both are
// throttled per (normalised) email using the shared in-process sliding-window
// limiter (`routes::rate_limit`) — the same algorithm the MFA verify surfaces
// use, keyed on `String` here instead of `user_id`.
//
// Keyed on email (not IP) because the abuse targets a specific mailbox, and it
// matches the login limiter's email keying (`SessionRepository::check_rate_limit`).
// Blocking on the email string leaks no account-existence signal: the 429 is a
// pure function of request volume for that string, independent of whether an
// account exists — so the anti-enumeration guarantee of these endpoints holds.
const AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS: u32 = 5;
const AUTH_EMAIL_DISPATCH_WINDOW: Duration = Duration::from_secs(900);
/// Only sweep expired limiter entries once the map exceeds this size.
const AUTH_EMAIL_DISPATCH_SWEEP_THRESHOLD: usize = 1024;

static FORGOT_PASSWORD_RATE_LIMITER: LazyLock<InProcessRateLimiter<String>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static RESEND_VERIFICATION_RATE_LIMITER: LazyLock<InProcessRateLimiter<String>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Normalise an email to its rate-limit key: trimmed + lowercased, matching the
/// `LOWER(email)` comparison the login limiter uses so casing/whitespace can't
/// be used to sidestep the throttle.
fn email_rate_key(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Record a `/forgot-password` attempt for `email`; `false` once more than
/// `AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS` land in a rolling window.
fn forgot_password_rate_allowed(email: &str) -> bool {
    rate_limit_allowed(
        &FORGOT_PASSWORD_RATE_LIMITER,
        email_rate_key(email),
        AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS,
        AUTH_EMAIL_DISPATCH_WINDOW,
        AUTH_EMAIL_DISPATCH_SWEEP_THRESHOLD,
    )
}

/// Record a `/resend-verification` attempt for `email`; `false` once more than
/// `AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS` land in a rolling window.
fn resend_verification_rate_allowed(email: &str) -> bool {
    rate_limit_allowed(
        &RESEND_VERIFICATION_RATE_LIMITER,
        email_rate_key(email),
        AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS,
        AUTH_EMAIL_DISPATCH_WINDOW,
        AUTH_EMAIL_DISPATCH_SWEEP_THRESHOLD,
    )
}

/// Shared 429 body for the email-dispatch surfaces. Kept generic (no
/// account-specific detail) so it does not become an enumeration oracle.
fn email_dispatch_rate_limited() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorResponse::new(
            "RATE_LIMITED",
            "Too many requests for this email. Please try again later.",
        )),
    )
}

/// Create auth router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/verify-email", get(verify_email))
        .route("/resend-verification", post(resend_verification))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        // Session management (Story 1.5)
        .route("/sessions", get(list_sessions))
        .route("/sessions/revoke", post(revoke_session))
        .route("/sessions/revoke-all", post(revoke_all_sessions))
        // Current user profile (TypeSpec auth.tsp `/me` GET + PATCH)
        .route("/me", get(get_me).patch(update_me))
}

mod password_reset;
mod registration;
mod sessions;

// Re-export each submodule's public handlers + DTO types so `routes::auth::<name>`
// references keep resolving unchanged after the per-surface split: the `router()`
// above, plus `paths(...)` / `components(schemas(...))` in `main.rs`. Glob
// re-exports also carry the utoipa-generated `__path_*` items next to each fn.
pub use password_reset::*;
pub use registration::*;
pub use sessions::*;

// ==================== Login (Story 1.2) ====================

/// Login request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// 2FA code (optional, for future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_code: Option<String>,
}

/// User's membership in a tenant/organization, as exposed to clients on
/// login and on token refresh. Mirrors the TypeSpec `TenantMembership`
/// model in `docs/api/typespec/domains/auth.tsp`.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantMembership {
    /// Tenant / organization ID.
    pub tenant_id: uuid::Uuid,
    /// Tenant / organization display name.
    pub tenant_name: String,
    /// User's role within this tenant (e.g. `manager`, `owner`).
    pub role: String,
}

/// Login response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    /// JWT access token (empty if MFA required)
    pub access_token: String,
    /// Refresh token (empty if MFA required)
    pub refresh_token: String,
    /// Access token expiration in seconds
    pub expires_in: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Whether MFA verification is required to complete login
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_required: Option<bool>,
    /// Temporary token for MFA verification (only present if mfa_required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_token: Option<String>,
    /// Active tenant memberships for this user, queried fresh from the
    /// database on every login and every token refresh. The frontend
    /// persists this list to back `deriveActiveRole`; returning it on
    /// refresh closes the staleness gap from #676 where a server-side
    /// membership revocation would not propagate until the next full
    /// re-login. Empty on the MFA-required intermediate response.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tenants: Vec<TenantMembership>,
}

/// Load the user's currently-active tenant memberships from the database.
///
/// "Active" here means `organization_members.status = 'active'` AND the
/// owning organization is not soft-deleted. The repository already filters
/// `status != 'removed'`; we additionally drop `pending` / `suspended` rows
/// here because those memberships must not grant access in the access-token
/// lifetime that the response unlocks.
///
/// This helper is the single source of truth for both `login()` and
/// `refresh_token()` so the two code paths cannot drift (#676).
async fn load_tenant_memberships(state: &AppState, user_id: uuid::Uuid) -> Vec<TenantMembership> {
    match state.org_member_repo.get_user_memberships(user_id).await {
        Ok(rows) => rows
            .into_iter()
            .filter(|row| row.status == "active")
            .map(|row| TenantMembership {
                tenant_id: row.organization_id,
                tenant_name: row.organization_name,
                role: row.role_type,
            })
            .collect(),
        Err(err) => {
            // Fail open with an empty list rather than 500 — the access
            // token itself is the authoritative authz signal; the tenants
            // list is a UX hint for the client to pick which tenant to act
            // as. Logging at warn so a persistent DB issue is still visible.
            tracing::warn!(
                error = %err,
                user_id = %user_id,
                "Failed to load tenant memberships; returning empty list"
            );
            Vec::new()
        }
    }
}

/// Login endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 429, description = "Too many failed attempts", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::response::IntoResponse;
    let ip_address = addr.ip().to_string();
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Check rate limiting
    match state.session_repo.check_rate_limit(&req.email).await {
        Ok(status) if !status.can_attempt() => {
            let remaining = status.lockout_remaining_secs.unwrap_or(900);
            tracing::warn!(
                email_hash = %common::email_log_hash(&req.email),
                ip = %ip_address,
                remaining_secs = remaining,
                "Login attempt blocked due to rate limiting"
            );
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse::new(
                    "RATE_LIMITED",
                    format!(
                        "Too many failed login attempts. Please try again in {} minutes.",
                        remaining / 60 + 1
                    ),
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check rate limit");
            // Continue anyway - don't block login due to rate limit check failure
        }
        _ => {}
    }

    // Find user by email
    let user = match state.user_repo.find_by_email(&req.email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Record failed attempt (user not found)
            let _ = state
                .session_repo
                .record_login_attempt(&req.email, &ip_address, false)
                .await;
            tracing::debug!(
                email_hash = %common::email_log_hash(&req.email),
                "Login failed: user not found"
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_CREDENTIALS",
                    "Invalid email or password",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Login failed")),
            ));
        }
    };

    // Check if user can log in
    if user.status == "suspended" {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "ACCOUNT_SUSPENDED",
                "Account suspended. Contact support.",
            )),
        ));
    }

    // Verify password
    let password_valid = match state
        .auth_service
        .verify_password(&req.password, &user.password_hash)
    {
        Ok(valid) => valid,
        Err(e) => {
            tracing::error!(error = %e, "Password verification error");
            let _ = state
                .session_repo
                .record_login_attempt(&req.email, &ip_address, false)
                .await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Login failed")),
            ));
        }
    };

    if !password_valid {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        tracing::debug!(
            user_id = %user.id,
            "Login failed: invalid password"
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_CREDENTIALS",
                "Invalid email or password",
            )),
        ));
    }

    // Email-verification gate. This MUST come *after* the password check:
    // returning EMAIL_NOT_VERIFIED before verifying the password turns login
    // into an account-enumeration oracle (an attacker learns an email is
    // registered-but-unverified without knowing the password). With a wrong
    // password the caller already got the generic INVALID_CREDENTIALS above
    // regardless of verification state (#956).
    if !user.is_verified() {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "EMAIL_NOT_VERIFIED",
                "Please verify your email first",
            )),
        ));
    }

    // Track whether the user has supplied & verified an MFA factor in this
    // request. Used by the per-org `AuthPolicyEnforcer::check_login` gate
    // below — an org policy can demand MFA-at-login for a role even if the
    // user has not yet enrolled a TOTP secret.
    let mut mfa_presented = false;

    // Check 2FA if enabled (Epic 9, Story 9.1)
    // Note: Login happens before RLS context is established. 2FA is user-level, not tenant-scoped.
    #[allow(deprecated)]
    let mfa_check_result = state.two_factor_repo.get_by_user_id(user.id).await;
    if let Ok(Some(mfa_record)) = mfa_check_result {
        if mfa_record.enabled {
            // 2FA is enabled - check if code was provided
            match &req.two_factor_code {
                Some(code) => {
                    // Decrypt secret if encrypted (Story 9.1 security fix)
                    let decrypted_secret = state
                        .totp_service
                        .decrypt_secret(&mfa_record.secret)
                        .map_err(|e| {
                            tracing::error!(error = %e, "Failed to decrypt TOTP secret");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ErrorResponse::new(
                                    "DECRYPTION_ERROR",
                                    "Failed to verify MFA code",
                                )),
                            )
                        })?;

                    // Verify TOTP code
                    let is_valid = state
                        .totp_service
                        .verify_code(&decrypted_secret, code)
                        .unwrap_or(false);

                    // If TOTP failed, try backup codes
                    let backup_codes: Vec<String> =
                        serde_json::from_value(mfa_record.backup_codes.clone()).unwrap_or_default();
                    let backup_result = if !is_valid {
                        state
                            .totp_service
                            .verify_backup_code(code, &backup_codes)
                            .ok()
                            .flatten()
                    } else {
                        None
                    };

                    if !is_valid && backup_result.is_none() {
                        let _ = state
                            .session_repo
                            .record_login_attempt(&req.email, &ip_address, false)
                            .await;
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ErrorResponse::new(
                                "INVALID_MFA_CODE",
                                "Invalid verification code",
                            )),
                        ));
                    }

                    // MFA factor verified (TOTP or backup code).
                    mfa_presented = true;

                    // If backup code was used, consume it and log it
                    if let Some(code_index) = backup_result {
                        // Note: Login happens before RLS context is established.
                        #[allow(deprecated)]
                        let _ = state
                            .two_factor_repo
                            .use_backup_code(user.id, code_index)
                            .await;

                        // Log backup code usage (Story 9.6 - Audit logging)
                        if let Err(e) = state
                            .audit_log_repo
                            .create(CreateAuditLog {
                                user_id: Some(user.id),
                                action: AuditAction::MfaBackupCodeUsed,
                                resource_type: Some("two_factor_auth".to_string()),
                                resource_id: Some(user.id),
                                org_id: None,
                                details: Some(serde_json::json!({ "code_index": code_index })),
                                old_values: None,
                                new_values: None,
                                ip_address: Some(ip_address.clone()),
                                user_agent: None,
                            })
                            .await
                        {
                            tracing::error!(error = %e, user_id = %user.id, "Failed to create audit log for backup code usage");
                        }

                        tracing::info!(user_id = %user.id, "Backup code used for login");
                    }
                }
                None => {
                    // No code provided - return MFA required response.
                    // The client should retry login with the two_factor_code included.
                    return Ok(Json(LoginResponse {
                        access_token: String::new(),
                        refresh_token: String::new(),
                        expires_in: 0,
                        token_type: "Bearer".to_string(),
                        mfa_required: Some(true),
                        mfa_token: None,
                        // Tenants are intentionally omitted on the MFA
                        // gate response: the client has no usable access
                        // token yet, so a tenant picker would be useless.
                        tenants: Vec::new(),
                    })
                    .into_response());
                }
            }
        }
    }

    // D2.2: enforce per-org auth policy at login. If ANY org the user has an
    // active membership in requires MFA for the user's role in that org and
    // the user has not presented an MFA factor in this request, refuse the
    // login with the same `mfa_required` 401 response shape the existing 2FA
    // path uses. Email verification is already enforced above by
    // `User::is_verified` so we map only `MfaRequired` here.
    let enforcer = crate::services::AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer.check_login(user.id, mfa_presented).await {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        match err {
            crate::services::AuthPolicyError::MfaRequired(role) => {
                tracing::info!(user_id = %user.id, role = %role, "Login blocked: org policy demands MFA for role");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "MFA_REQUIRED",
                        format!("MFA required by org policy for role '{}'", role),
                    )),
                ));
            }
            other => {
                tracing::error!(error = %other, user_id = %user.id, "Auth policy check at login failed");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("AUTH_POLICY_ERROR", "Login failed")),
                ));
            }
        }
    }

    // Record successful login attempt
    let _ = state
        .session_repo
        .record_login_attempt(&req.email, &ip_address, true)
        .await;

    // Generate access token, embedding principal_kind (Phase 6 C17).
    let access_token = match state.jwt_service.generate_access_token_with_kind(
        user.id,
        &user.email,
        &user.name,
        None, // org_id - will be set when org context is selected
        None, // roles - will be set when org context is selected
        Some(user.principal_kind.clone()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate access token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_ERROR",
                    "Failed to create session",
                )),
            ));
        }
    };

    // Generate refresh token
    let (refresh_token, token_hash, expires_at) =
        match state
            .jwt_service
            .generate_refresh_token(user.id, &user.email, &user.name)
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = %e, "Failed to generate refresh token");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "TOKEN_ERROR",
                        "Failed to create session",
                    )),
                ));
            }
        };

    // Store refresh token in database
    use db::models::CreateRefreshToken;
    let create_token = CreateRefreshToken {
        user_id: user.id,
        token_hash,
        expires_at,
        user_agent: user_agent.clone(),
        ip_address: Some(ip_address.clone()),
        device_info: None, // Can be set from client-provided header in future
    };

    if let Err(e) = state.session_repo.create_refresh_token(create_token).await {
        tracing::error!(error = %e, "Failed to store refresh token");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create session",
            )),
        ));
    }

    tracing::info!(
        user_id = %user.id,
        "User logged in successfully"
    );

    // P0-12 / gap-security-435-cookie-scope: emit an HttpOnly Secure
    // SameSite=Strict cookie bearing the refresh token, alongside the JSON
    // body refresh_token for back-compat. Once the frontend is updated to
    // send `withCredentials: true` on /auth/refresh and stops storing the
    // refresh in localStorage, the body field can be dropped. Scoped to
    // /api/v1/auth so it isn't sent on unrelated API calls.
    // Issue #438: build_refresh_cookie now returns Result; a malformed
    // token character would previously panic the login handler.
    let mut headers = axum::http::HeaderMap::new();
    match build_refresh_cookie(
        &refresh_token,
        /*max_age_seconds=*/ 7 * 24 * 60 * 60,
        std::env::var("PPT_AUTH_COOKIE_SAMESITE").ok().as_deref(),
    )
    .and_then(|c| axum::http::HeaderValue::from_str(&c).map_err(|_| "non-ASCII cookie"))
    {
        Ok(hv) => {
            headers.append(axum::http::header::SET_COOKIE, hv);
        }
        Err(e) => {
            tracing::error!(
                error = e,
                "Skipping refresh-token Set-Cookie header (malformed value)"
            );
        }
    }

    // Issue #676: populate `tenants` with a fresh DB read so the frontend
    // can persist an accurate membership list. Login and refresh share the
    // same `load_tenant_memberships` helper to prevent drift.
    let tenants = load_tenant_memberships(&state, user.id).await;

    Ok((
        headers,
        Json(LoginResponse {
            access_token,
            refresh_token,
            expires_in: state.jwt_service.access_token_lifetime(),
            token_type: "Bearer".to_string(),
            mfa_required: None,
            mfa_token: None,
            tenants,
        }),
    )
        .into_response())
}

/// Build the standard `refresh_token` cookie attributes. Centralized so
/// login, refresh, and logout all agree on Path/SameSite/Secure flags.
///
/// ## Security attributes (P0-12 / gap-security-435-cookie-scope)
///
/// | Attribute    | Value / Source                     | Rationale                                  |
/// |-------------|------------------------------------|--------------------------------------------|
/// | `HttpOnly`  | always set                         | Prevents JS from reading the token.        |
/// | `Secure`    | always set                         | Cookie only sent over HTTPS.              |
/// | `SameSite`  | `Strict` (default) or env override | Strict stops CSRF cross-site requests.     |
/// |             | (`PPT_AUTH_COOKIE_SAMESITE`)       | Override to `Lax` only if a server-side    |
/// |             |                                    | OAuth/SSO redirect-back flow is deployed   |
/// |             |                                    | on the same origin as the api-server.      |
/// | `Path`      | `/api/v1/auth`                     | Cookie only sent to auth endpoints —       |
/// |             |                                    | not on every API call (scope limited).     |
///
/// `Domain` is optional and read from `PPT_AUTH_COOKIE_DOMAIN`; omit when
/// unset (back-compat — the cookie stays host-bound to the API origin).
/// Build a `Set-Cookie` value for the `refresh_token` cookie.
///
/// `same_site_override` — if `Some`, used directly (caller is responsible for
/// reading `PPT_AUTH_COOKIE_SAMESITE` from the environment and passing it here).
/// If `None`, defaults to `"Strict"`.  Accepting the override as a parameter
/// keeps env-var reads out of the function body and makes the unit tests
/// deterministic without `std::env::set_var` (which is not safe under parallel
/// `cargo test`).
fn build_refresh_cookie(
    value: &str,
    max_age_seconds: i64,
    same_site_override: Option<&str>,
) -> Result<String, &'static str> {
    // Issue #438: reject token characters that would let a malformed value
    // inject extra cookie attributes (`; Domain=.attacker.com`) or break
    // the HTTP header framing (`\r\n`). Also reject non-ASCII so the
    // downstream `HeaderValue::from_str` never panics — the previous
    // `.expect(...)` was a silent-DoS vector on the login/logout path.
    // Empty value is allowed (clear-cookie path uses max_age=0).
    if !value.is_empty()
        && !value.bytes().all(|b| {
            // RFC 6265 cookie-octet: visible US-ASCII except `;` `,` `"` `\` and whitespace.
            (0x21..=0x7E).contains(&b) && b != b';' && b != b',' && b != b'"' && b != b'\\'
        })
    {
        return Err("invalid refresh-token characters for Set-Cookie");
    }
    // P0-12 (gap-security-435-cookie-scope): default is now Strict.  Operators
    // running an OAuth/SSO redirect-back flow on the same origin can override to
    // Lax via PPT_AUTH_COOKIE_SAMESITE=Lax.  The previous default was Lax which
    // left an unnecessary CSRF surface for most deployments.
    // filter rejects any value that is not one of the three RFC 6265 SameSite tokens;
    // invalid env values fall back to "Strict" rather than being forwarded verbatim.
    let same_site = same_site_override
        .filter(|s| matches!(*s, "Strict" | "Lax" | "None"))
        .unwrap_or("Strict");
    let mut cookie = format!(
        "refresh_token={value}; HttpOnly; Secure; SameSite={same_site}; Path=/api/v1/auth; Max-Age={max_age_seconds}"
    );
    if let Ok(domain) = std::env::var("PPT_AUTH_COOKIE_DOMAIN") {
        let domain = domain.trim();
        if !domain.is_empty() {
            if domain
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
            {
                cookie.push_str("; Domain=");
                cookie.push_str(domain);
            } else {
                tracing::warn!("PPT_AUTH_COOKIE_DOMAIN contains invalid characters — ignoring");
            }
        }
    }
    Ok(cookie)
}

/// Parse the `refresh_token` cookie out of the `Cookie:` header. Returns
/// `None` if the header is absent, the cookie name isn't present, or the
/// cookie is present but empty (e.g. `refresh_token=;`). Treating an empty
/// cookie as absent is deliberate: a present-but-empty cookie must NOT
/// shadow a valid token supplied in the request body — callers use
/// `.unwrap_or(body_token)`, so returning `Some("")` here would silently
/// override a legitimate body/header token and break `/refresh` and
/// `/logout`. Deliberately permissive whitespace handling otherwise — we
/// don't validate the token shape here; the JWT validation downstream
/// rejects garbage.
fn parse_refresh_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("refresh_token=") {
            if rest.is_empty() {
                return None;
            }
            return Some(rest.to_string());
        }
    }
    None
}

/// Resolve the refresh token for `/refresh` and `/logout`, preferring the
/// HttpOnly `refresh_token` cookie and falling back to the value supplied in
/// the JSON body.
///
/// Centralizes the cookie-first / body-fallback precedence that both
/// `refresh_token()` and `logout()` previously hand-rolled as
/// `parse_refresh_cookie(&headers).as_deref().unwrap_or(body)`. Keeping the two
/// paths on one helper is what prevents the empty-cookie-shadow class of bug
/// (#2205) from being fixed in one handler but not the other: because
/// `parse_refresh_cookie` already treats a present-but-empty `refresh_token=`
/// cookie as absent (returns `None`), resolution correctly falls through to
/// `body_token` here.
fn resolve_refresh_token(headers: &axum::http::HeaderMap, body_token: &str) -> String {
    parse_refresh_cookie(headers).unwrap_or_else(|| body_token.to_string())
}

/// SHA-256 hash (hex-encoded) of a refresh token, matching the digest stored in
/// `refresh_tokens.token_hash`.
///
/// Centralized so login / refresh / logout / session-lookup all hash
/// identically — this was hand-rolled as three byte-for-byte copies of the
/// `Sha256::new(); update(token.as_bytes()); hex::encode(finalize())` dance
/// that all had to stay in sync.
fn hash_refresh_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Resolve the caller's *current* refresh-token session id.
///
/// Bug fix (revoke-all cookie blindness): after the P0-12 cookie migration,
/// ppt-web sends the refresh token only via the `HttpOnly` `refresh_token`
/// cookie — never the `X-Refresh-Token` header. The `list_sessions` and
/// `revoke_all_sessions` handlers previously read the header only, so
/// cookie-based callers resolved to `None`. For `revoke_all_sessions` that
/// meant `revoke_all_user_tokens(user_id, None)` revoked the caller's *own*
/// live session ("sign out other devices" signed YOU out); for
/// `list_sessions` it meant `isCurrent` could never be `true`.
///
/// Mirror the cookie-first / header-fallback precedence already used by
/// `refresh_token` (line ~1077) and `logout`: prefer the cookie, fall back to
/// the header, hash the token, and look the session up. Header-only callers
/// (mobile RN, older ppt-web builds) keep working unchanged because the header
/// fallback still runs.
async fn resolve_current_session_id(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    user_id: uuid::Uuid,
) -> Option<uuid::Uuid> {
    let token = parse_refresh_cookie(headers).or_else(|| {
        headers
            .get("X-Refresh-Token")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    })?;
    if token.is_empty() {
        return None;
    }

    let token_hash = hash_refresh_token(&token);

    // Defense-in-depth: `find_by_token_hash` resolves *any* user's live
    // session by hash. Only treat it as the caller's current session when it
    // actually belongs to the authenticated user — a stray/stale cookie
    // belonging to a different user must not become `except_id` (which would
    // resurface the "sign out other devices signs YOU out" failure) nor mark a
    // foreign session `isCurrent`.
    state
        .session_repo
        .find_by_token_hash(&token_hash)
        .await
        .ok()
        .flatten()
        .filter(|session| session.user_id == user_id)
        .map(|session| session.id)
}

// ==================== Refresh Token (Story 1.3) ====================

/// Refresh token request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    /// The refresh token to exchange for new tokens
    pub refresh_token: String,
}

/// Refresh token endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed", body = LoginResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse)
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // P0-12 (additive): prefer the HttpOnly cookie over the JSON body
    // when present. localStorage-based clients still send the token
    // in the body; cookie-based clients send only the cookie and an
    // empty body. Once all clients are migrated, the body field can
    // be removed.
    let token_str = resolve_refresh_token(&headers, &req.refresh_token);
    if token_str.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "MISSING_TOKEN",
                "Refresh token not provided in cookie or body",
            )),
        ));
    }

    // Validate the refresh token JWT
    let claims = match state.jwt_service.validate_refresh_token(&token_str) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::debug!(error = %e, "Invalid refresh token");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired refresh token",
                )),
            ));
        }
    };

    // Hash the token to look it up in database
    let token_hash = hash_refresh_token(&token_str);

    // P1-01: refresh-token rotation with replay detection (RFC 9700).
    // Look up the token *regardless of revocation status* first. If the
    // hash matches an already-revoked row, the legitimate user has long
    // since rotated past it — a presenter of that token is either an
    // attacker replaying a stolen copy OR a confused client that lagged
    // a rotation. In either case the safe response is to revoke every
    // active refresh token for the user (we don't have a family_id
    // column, so we approximate the family by "all the user's tokens")
    // and write a security audit row so SOC tooling sees the event.
    let stored_token = match state
        .session_repo
        .find_by_token_hash_any_status(&token_hash)
        .await
    {
        Ok(Some(token)) if token.revoked_at.is_some() => {
            let user_id_for_log = token.user_id;
            tracing::warn!(
                user_id = %user_id_for_log,
                token_id = %token.id,
                "Revoked refresh token replayed; invalidating all user refresh tokens"
            );
            // Issue #439 P1-01: write the audit row FIRST so a downstream
            // revoke failure cannot rob us of the security signal. Mirror
            // the P1-09 pattern: surface audit-write errors via
            // `tracing::warn!` instead of `let _ = ...`.
            if let Err(err) = state
                .audit_log_repo
                .create(CreateAuditLog {
                    user_id: Some(user_id_for_log),
                    action: AuditAction::RefreshTokenReplayDetected,
                    resource_type: Some("refresh_tokens".to_string()),
                    resource_id: Some(token.id),
                    org_id: None,
                    details: Some(serde_json::json!({
                        "reason": "revoked_token_replayed",
                        "remediation": "all_user_refresh_tokens_revoked",
                    })),
                    old_values: None,
                    new_values: None,
                    ip_address: None,
                    user_agent: None,
                })
                .await
            {
                tracing::warn!(
                    error = %err,
                    user_id = %user_id_for_log,
                    "audit row missed for refresh-replay"
                );
            }
            // Best-effort fan-out revocation. Even if this fails the
            // caller still gets 401, but we want the security log row.
            if let Err(err) = state
                .session_repo
                .revoke_all_user_tokens(user_id_for_log, None)
                .await
            {
                tracing::error!(
                    error = %err,
                    user_id = %user_id_for_log,
                    "Failed to fan-out-revoke after refresh-token replay"
                );
            }
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "TOKEN_REVOKED",
                    "This session has been revoked",
                )),
            ));
        }
        Ok(Some(token)) => token, // active path
        Ok(None) => {
            tracing::warn!(user_id = %claims.sub, "Refresh token not found in database");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "TOKEN_REVOKED",
                    "This session has been revoked",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding refresh token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to validate session",
                )),
            ));
        }
    };

    // Check if token is still valid
    if !stored_token.is_valid() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("TOKEN_EXPIRED", "Session has expired")),
        ));
    }

    // Parse user ID from claims
    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Verify user still exists and is active
    let user = match state.user_repo.find_by_id(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User account no longer exists",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to validate user",
                )),
            ));
        }
    };

    if user.status != "active" {
        // Revoke the token since user is no longer active
        let _ = state.session_repo.revoke_token(stored_token.id).await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "ACCOUNT_INACTIVE",
                "Account is no longer active",
            )),
        ));
    }

    // Token rotation: revoke the old token
    if let Err(e) = state.session_repo.revoke_token(stored_token.id).await {
        tracing::error!(error = %e, "Failed to revoke old refresh token");
        // Continue anyway - better to issue new token than fail
    }

    // Generate new access token, re-embedding principal_kind (Phase 6 C17).
    let access_token = match state.jwt_service.generate_access_token_with_kind(
        user.id,
        &user.email,
        &user.name,
        None,
        None,
        Some(user.principal_kind.clone()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate access token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_ERROR",
                    "Failed to create session",
                )),
            ));
        }
    };

    // Generate new refresh token (rotation)
    let (new_refresh_token, new_token_hash, expires_at) = match state
        .jwt_service
        .generate_refresh_token(user.id, &user.email, &user.name)
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate refresh token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_ERROR",
                    "Failed to create session",
                )),
            ));
        }
    };

    // Store new refresh token
    use db::models::CreateRefreshToken;
    let create_token = CreateRefreshToken {
        user_id: user.id,
        token_hash: new_token_hash,
        expires_at,
        user_agent: stored_token.user_agent.clone(),
        ip_address: stored_token.ip_address.clone(),
        device_info: stored_token.device_info.clone(),
    };

    if let Err(e) = state.session_repo.create_refresh_token(create_token).await {
        tracing::error!(error = %e, "Failed to store new refresh token");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create session",
            )),
        ));
    }

    tracing::info!(user_id = %user.id, "Token refreshed successfully");

    // Issue #676: re-read tenant memberships from the database on every
    // refresh. The frontend persists this list at login and, prior to
    // this fix, had no way to learn that a membership was revoked
    // server-side until the user logged out and back in — letting
    // `deriveActiveRole` keep handing back a removed role for up to the
    // refresh-token lifetime. Sharing `load_tenant_memberships` with
    // `login()` keeps the two paths in lockstep.
    let tenants = load_tenant_memberships(&state, user.id).await;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: state.jwt_service.access_token_lifetime(),
        token_type: "Bearer".to_string(),
        mfa_required: None,
        mfa_token: None,
        tenants,
    }))
}

// ==================== Logout (Story 1.3) ====================

/// Logout request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    /// The refresh token to revoke
    pub refresh_token: String,
}

/// Logout response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {
    /// Success message
    pub message: String,
}

/// Logout endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse),
        (status = 401, description = "Invalid token")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LogoutRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::response::IntoResponse;
    // P0-12 (additive): accept cookie too.
    let token_str = resolve_refresh_token(&headers, &req.refresh_token);

    // Hash the token to look it up
    let token_hash = hash_refresh_token(&token_str);

    // Find and revoke the token
    match state.session_repo.find_by_token_hash(&token_hash).await {
        Ok(Some(token)) => {
            if let Err(e) = state.session_repo.revoke_token(token.id).await {
                tracing::error!(error = %e, "Failed to revoke token");
            } else {
                tracing::info!(user_id = %token.user_id, "User logged out");
            }
        }
        Ok(None) => {
            // Token not found - might already be revoked, that's fine
            tracing::debug!("Logout requested for unknown/revoked token");
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during logout");
        }
    }

    // Always return success to prevent token enumeration. Clear the
    // refresh cookie by setting Max-Age=0 with the same attributes.
    let mut response_headers = axum::http::HeaderMap::new();
    if let Ok(hv) = build_refresh_cookie(
        "",
        0,
        std::env::var("PPT_AUTH_COOKIE_SAMESITE").ok().as_deref(),
    )
    .and_then(|c| axum::http::HeaderValue::from_str(&c).map_err(|_| "non-ASCII cookie"))
    {
        response_headers.append(axum::http::header::SET_COOKIE, hv);
    }
    Ok((
        response_headers,
        Json(LogoutResponse {
            message: "Logged out successfully".to_string(),
        }),
    )
        .into_response())
}

// ==================== Current User Profile (TypeSpec `/me`) ====================

/// Public-facing user shape returned by `GET /me` and `PATCH /me`.
///
/// Mirrors the `Auth.User` model in `docs/api/typespec/domains/auth.tsp`. Fields
/// are camelCase to match the TypeSpec contract and the generated TS SDK
/// (`Auth_User` in `frontend/packages/api-client/src/generated/models.ts`).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserResponse {
    /// Unique identifier
    pub id: String,
    /// Email address
    pub email: String,
    /// Display name
    pub display_name: String,
    /// Phone number (E.164 or local format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Profile picture URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Whether email is verified
    pub email_verified: bool,
    /// Whether 2FA is enabled
    pub two_factor_enabled: bool,
    /// Last login timestamp (RFC3339). Not currently tracked on `users`, so
    /// this is omitted in responses until the column is added.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    /// Account status (`active` | `inactive` | `suspended` | `pending_verification`)
    pub status: String,
    /// Creation timestamp (RFC3339)
    pub created_at: String,
    /// Last update timestamp (RFC3339)
    pub updated_at: String,
}

impl AuthUserResponse {
    /// Build response from a DB `User` row + the user's 2FA-enabled flag.
    fn from_user(user: &db::models::User, two_factor_enabled: bool) -> Self {
        // Map internal status to TypeSpec `UserStatus`. The DB stores
        // `pending` for unverified accounts; TypeSpec calls that
        // `pending_verification`.
        let status = match user.status.as_str() {
            "pending" => "pending_verification".to_string(),
            other => other.to_string(),
        };
        Self {
            id: user.id.to_string(),
            email: user.email.clone(),
            display_name: user.name.clone(),
            phone: user.phone.clone(),
            avatar_url: user.profile_image_url.clone(),
            email_verified: user.is_verified(),
            two_factor_enabled,
            last_login_at: None,
            status,
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
    }
}

/// Body for `PATCH /api/v1/auth/me`.
///
/// **Whitelist only**: `displayName`, `phone`, `avatarUrl`. Identity fields
/// (`id`, `email`, `role`, `tenant_id`, `is_admin`, `password`) and the
/// `status` flag are intentionally NOT accepted — they have dedicated
/// verification/admin flows.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeRequest {
    /// New display name (1..=120 chars, non-blank).
    #[serde(default)]
    pub display_name: Option<String>,
    /// New phone number, or `null`/omit to keep current.
    #[serde(default)]
    pub phone: Option<String>,
    /// New profile picture URL.
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// Get current authenticated user's profile.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user profile", body = AuthUserResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AuthUserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = authenticated_user_id(&state, &headers)?;

    let user = match state.user_repo.find_by_id(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User account no longer exists",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error loading current user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to load profile",
                )),
            ));
        }
    };

    // `two_factor_enabled` is a user-level flag, not tenant-scoped. Login
    // already reads it the same way; mirror that for `/me`.
    #[allow(deprecated)]
    let two_factor_enabled = state
        .two_factor_repo
        .get_by_user_id(user.id)
        .await
        .ok()
        .flatten()
        .map(|r| r.enabled)
        .unwrap_or(false);

    Ok(Json(AuthUserResponse::from_user(&user, two_factor_enabled)))
}

/// Update current authenticated user's profile (whitelist of profile-y fields).
#[utoipa::path(
    patch,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = UpdateMeRequest,
    responses(
        (status = 200, description = "Profile updated", body = AuthUserResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<UpdateMeRequest>,
) -> Result<Json<AuthUserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = authenticated_user_id(&state, &headers)?;

    // Validate displayName (if provided) — must be non-blank, <= 120 chars.
    if let Some(name) = req.display_name.as_ref() {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_DISPLAY_NAME",
                    "displayName cannot be empty",
                )),
            ));
        }
        if trimmed.chars().count() > 120 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_DISPLAY_NAME",
                    "displayName exceeds 120 characters",
                )),
            ));
        }
    }

    // Reject empty patches early — nothing to do.
    if req.display_name.is_none() && req.phone.is_none() && req.avatar_url.is_none() {
        // Idempotent no-op: return the current profile rather than 400.
        return get_me(State(state), headers).await;
    }

    let update = UpdateUser {
        name: req.display_name.as_ref().map(|s| s.trim().to_string()),
        phone: req.phone.clone(),
        locale: None,
        avatar_url: req.avatar_url.clone(),
    };

    let user = match state.user_repo.update(user_id, update).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User account no longer exists",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, "Failed to update profile");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update profile",
                )),
            ));
        }
    };

    #[allow(deprecated)]
    let two_factor_enabled = state
        .two_factor_repo
        .get_by_user_id(user.id)
        .await
        .ok()
        .flatten()
        .map(|r| r.enabled)
        .unwrap_or(false);

    tracing::info!(user_id = %user.id, "Profile updated via PATCH /me");
    Ok(Json(AuthUserResponse::from_user(&user, two_factor_enabled)))
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

/// Authenticate a bearer-token request and return the caller's user id.
///
/// Runs the identical three-step preamble every bearer-authenticated handler in
/// this module (`/sessions`, `/sessions/revoke`, `/sessions/revoke-all`,
/// `GET /me`, `PATCH /me`) previously hand-rolled: extract the
/// `Authorization: Bearer` token, validate it, and parse the `sub` claim into a
/// `Uuid`. Centralizing it keeps the error responses (401 `MISSING_TOKEN` /
/// `INVALID_TOKEN`) byte-for-byte identical across those handlers and removes
/// five verbatim copies of the `claims.sub.parse()` block.
fn authenticated_user_id(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<uuid::Uuid, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(headers)?;
    let claims = validate_access_token(state, &token)?;
    claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })
}

#[cfg(test)]
mod me_tests {
    use super::{AuthUserResponse, UpdateMeRequest};
    use serde_json::json;

    /// `AuthUserResponse` must serialize in camelCase to match the TypeSpec
    /// `Auth.User` contract and the generated TS SDK (`Auth_User`).
    #[test]
    fn auth_user_response_uses_camelcase() {
        let resp = AuthUserResponse {
            id: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "alice@example.com".to_string(),
            display_name: "Alice".to_string(),
            phone: Some("+421900000000".to_string()),
            avatar_url: Some("https://cdn/a.png".to_string()),
            email_verified: true,
            two_factor_enabled: false,
            last_login_at: None,
            status: "active".to_string(),
            created_at: "2026-05-18T00:00:00+00:00".to_string(),
            updated_at: "2026-05-18T00:00:00+00:00".to_string(),
        };
        let v = serde_json::to_value(&resp).expect("serialize");
        // camelCase keys present
        for key in [
            "id",
            "email",
            "displayName",
            "phone",
            "avatarUrl",
            "emailVerified",
            "twoFactorEnabled",
            "status",
            "createdAt",
            "updatedAt",
        ] {
            assert!(v.get(key).is_some(), "missing camelCase key: {}", key);
        }
        // snake_case must not leak
        for bad in ["display_name", "avatar_url", "email_verified", "created_at"] {
            assert!(v.get(bad).is_none(), "leaked snake_case key: {}", bad);
        }
        // optional `lastLoginAt` skipped when None
        assert!(v.get("lastLoginAt").is_none());
    }

    /// PATCH body must accept camelCase from the SDK.
    #[test]
    fn update_me_request_parses_camelcase() {
        let body = json!({
            "displayName": "Bob",
            "phone": "+421900000001",
            "avatarUrl": "https://cdn/b.png"
        });
        let parsed: UpdateMeRequest = serde_json::from_value(body).expect("parse");
        assert_eq!(parsed.display_name.as_deref(), Some("Bob"));
        assert_eq!(parsed.phone.as_deref(), Some("+421900000001"));
        assert_eq!(parsed.avatar_url.as_deref(), Some("https://cdn/b.png"));
    }

    /// PATCH body must reject identity fields silently — they are not declared
    /// in `UpdateMeRequest`, so serde drops unknown keys and the whitelist
    /// holds. (Validates the "do not allow updating id/email/role/password"
    /// requirement at the type level.)
    #[test]
    fn update_me_request_ignores_unknown_fields() {
        let body = json!({
            "displayName": "Carol",
            "email": "attacker@example.com",
            "role": "admin",
            "tenantId": "00000000-0000-0000-0000-000000000000",
            "isAdmin": true,
            "password": "hunter2"
        });
        let parsed: UpdateMeRequest = serde_json::from_value(body).expect("parse");
        assert_eq!(parsed.display_name.as_deref(), Some("Carol"));
        assert!(parsed.phone.is_none());
        assert!(parsed.avatar_url.is_none());
    }
}

// ==================== Cookie security unit tests (P0-12) ====================

#[cfg(test)]
mod cookie_security_tests {
    use super::{
        build_refresh_cookie, hash_refresh_token, parse_refresh_cookie, resolve_refresh_token,
    };

    /// P0-12 / gap-security-435-cookie-scope: default SameSite must be Strict.
    ///
    /// Passing `None` as `same_site_override` exercises the default code path
    /// without touching `std::env` (which is not safe under parallel `cargo test`).
    #[test]
    fn refresh_cookie_default_samesite_is_strict() {
        let cookie = build_refresh_cookie("tok.en_VALUE-test", 3600, None)
            .expect("valid token should not error");
        assert!(
            cookie.contains("SameSite=Strict"),
            "Expected SameSite=Strict in cookie, got: {cookie}"
        );
    }

    /// `HttpOnly` must always be present.
    #[test]
    fn refresh_cookie_always_httponly() {
        let cookie = build_refresh_cookie("tok", 3600, None).expect("valid");
        assert!(
            cookie.contains("HttpOnly"),
            "Missing HttpOnly flag: {cookie}"
        );
    }

    /// `Secure` must always be present.
    #[test]
    fn refresh_cookie_always_secure() {
        let cookie = build_refresh_cookie("tok", 3600, None).expect("valid");
        assert!(cookie.contains("Secure"), "Missing Secure flag: {cookie}");
    }

    /// Path must be scoped to `/api/v1/auth` — not `/` or a broader prefix.
    #[test]
    fn refresh_cookie_path_scoped_to_auth() {
        let cookie = build_refresh_cookie("tok", 3600, None).expect("valid");
        assert!(
            cookie.contains("Path=/api/v1/auth"),
            "Expected Path=/api/v1/auth in cookie, got: {cookie}"
        );
        // Ensure the path is NOT the root path.
        assert!(
            !cookie.contains("Path=/;") && !cookie.ends_with("Path=/"),
            "Cookie path must not be root /: {cookie}"
        );
    }

    /// Passing `Some("Lax")` must produce `SameSite=Lax` (SSO deployment override).
    #[test]
    fn refresh_cookie_env_override_lax() {
        let cookie = build_refresh_cookie("tok", 3600, Some("Lax")).expect("valid");
        assert!(
            cookie.contains("SameSite=Lax"),
            "Expected SameSite=Lax with override, got: {cookie}"
        );
    }

    /// An unrecognised override value must fall back to `Strict` (not Lax).
    #[test]
    fn refresh_cookie_invalid_env_falls_back_to_strict() {
        let cookie = build_refresh_cookie("tok", 3600, Some("garbage-value")).expect("valid");
        assert!(
            cookie.contains("SameSite=Strict"),
            "Expected SameSite=Strict fallback for invalid override, got: {cookie}"
        );
    }

    /// Clear-cookie (max_age=0, value="") must still carry all security attributes.
    #[test]
    fn clear_refresh_cookie_has_all_security_flags() {
        let cookie = build_refresh_cookie("", 0, None).expect("valid");
        assert!(cookie.contains("HttpOnly"), "Missing HttpOnly: {cookie}");
        assert!(cookie.contains("Secure"), "Missing Secure: {cookie}");
        assert!(
            cookie.contains("SameSite=Strict"),
            "Missing SameSite=Strict: {cookie}"
        );
        assert!(cookie.contains("Path=/api/v1/auth"), "Wrong path: {cookie}");
        assert!(cookie.contains("Max-Age=0"), "Expected Max-Age=0: {cookie}");
    }

    /// Tokens with injection characters must be rejected.
    #[test]
    fn refresh_cookie_rejects_semicolon_injection() {
        let result = build_refresh_cookie("valid; Expires=0", 3600, None);
        assert!(result.is_err(), "Should reject semicolon in token");
    }

    /// `parse_refresh_cookie` must extract the token by name from a Cookie header.
    #[test]
    fn parse_refresh_cookie_extracts_token() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            "other=x; refresh_token=my.jwt.token; more=y"
                .parse()
                .unwrap(),
        );
        let token = parse_refresh_cookie(&headers);
        assert_eq!(token.as_deref(), Some("my.jwt.token"));
    }

    /// Absent `refresh_token` cookie must return `None`.
    #[test]
    fn parse_refresh_cookie_absent_returns_none() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            "session_id=abc".parse().unwrap(),
        );
        let token = parse_refresh_cookie(&headers);
        assert!(token.is_none());
    }

    // ==================== Path-reconciliation tests (#617) ====================
    //
    // Issue #617: PR #565 changed the cookie Path from `/` to `/api/v1/auth`.
    // These tests verify:
    //   1. The Set-Cookie Path is exactly `/api/v1/auth` with no trailing slash.
    //      A trailing slash would make the Path `/api/v1/auth/` — the browser
    //      would NOT send the cookie to `/api/v1/auth/login` on some user agents,
    //      silently breaking login/refresh for cookie-first clients.
    //   2. The clear-cookie (Max-Age=0) uses the SAME Path as the set-cookie.
    //      If they differ, the browser stores two cookies (one live, one expired)
    //      and the logout handler cannot clear the live one — silent no-op logout.
    //   3. Old sessions that hold the token in the JSON body (localStorage flow)
    //      are still accepted because `refresh_token` fallback to the body is
    //      preserved in the handler.  That path is exercised via
    //      `parse_refresh_cookie` returning `None` when no cookie is present.

    /// Path must be exactly `/api/v1/auth` — no trailing slash.
    ///
    /// Trailing slash would produce `Path=/api/v1/auth/` which some browsers
    /// treat as NOT matching `/api/v1/auth/login` (without the slash), causing
    /// a silent cookie miss on every auth call.
    #[test]
    fn refresh_cookie_path_has_no_trailing_slash() {
        let cookie = build_refresh_cookie("tok", 3600, None).expect("valid");
        // The Path attribute as written must be exactly `/api/v1/auth`.
        // We check that the literal string `Path=/api/v1/auth;` (with semicolon
        // or end-of-string) appears — no `Path=/api/v1/auth/` variant.
        let path_attr = cookie
            .split(';')
            .find(|seg| seg.trim().starts_with("Path="))
            .expect("Path attribute must be present");
        assert_eq!(
            path_attr.trim(),
            "Path=/api/v1/auth",
            "Cookie Path must be exactly /api/v1/auth (no trailing slash): {cookie}"
        );
    }

    /// Set-cookie and clear-cookie must use the SAME Path value.
    ///
    /// If the paths differ the browser keeps both cookies and the logout
    /// `Max-Age=0` clear-cookie only expires the one it matches — the live
    /// session cookie survives, causing a silent no-op logout (issue #617).
    #[test]
    fn set_and_clear_cookie_use_identical_path() {
        let set_cookie = build_refresh_cookie("tok.value", 604800, None).expect("valid set");
        let clear_cookie = build_refresh_cookie("", 0, None).expect("valid clear");

        let extract_path = |c: &str| -> String {
            c.split(';')
                .find(|seg| seg.trim().starts_with("Path="))
                .map(|seg| seg.trim().to_string())
                .unwrap_or_default()
        };

        let set_path = extract_path(&set_cookie);
        let clear_path = extract_path(&clear_cookie);
        assert_eq!(
            set_path, clear_path,
            "Set-cookie and clear-cookie must use the same Path; set={set_cookie}, clear={clear_cookie}"
        );
    }

    /// A request with no `refresh_token` cookie falls back gracefully (`None`).
    ///
    /// This verifies that the body-based fallback is still reachable for
    /// existing sessions that pre-date the P0-12 cookie migration and have
    /// their refresh token stored in localStorage / sent in the JSON body.
    #[test]
    fn parse_refresh_cookie_returns_none_when_cookie_header_absent() {
        // No Cookie header at all — simulates a pre-migration client.
        let headers = axum::http::HeaderMap::new();
        let result = parse_refresh_cookie(&headers);
        assert!(
            result.is_none(),
            "Expected None for request without Cookie header (body-fallback path): {result:?}"
        );
    }

    /// `parse_refresh_cookie` must not confuse a similarly-named cookie
    /// (e.g. `other_refresh_token=x`) for the canonical `refresh_token`.
    #[test]
    fn parse_refresh_cookie_requires_exact_name_match() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            "other_refresh_token=fake; not_refresh_token=also_fake"
                .parse()
                .unwrap(),
        );
        let result = parse_refresh_cookie(&headers);
        assert!(
            result.is_none(),
            "Should not match cookie names that merely contain 'refresh_token': {result:?}"
        );
    }

    /// Regression: a present-but-EMPTY `refresh_token` cookie must NOT shadow
    /// a valid token supplied in the request body / `X-Refresh-Token` header.
    ///
    /// Both `refresh_token()` and `logout()` resolve the token as
    /// `parse_refresh_cookie(&headers).as_deref().unwrap_or(body_token)`.
    /// Before the fix, `parse_refresh_cookie` returned `Some("")` for a
    /// `refresh_token=;` cookie, so `unwrap_or` kept the empty string and the
    /// legitimate body token was ignored — `/refresh` returned MISSING_TOKEN
    /// and `/logout` hashed the empty string, silently failing to revoke.
    /// `parse_refresh_cookie` must now return `None` for an empty cookie so
    /// the body/header token wins.
    #[test]
    fn parse_refresh_cookie_empty_cookie_does_not_shadow_body_token() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            "session_id=abc; refresh_token=; other=y".parse().unwrap(),
        );

        // The cookie is present but empty -> must be treated as absent.
        let cookie_token = parse_refresh_cookie(&headers);
        assert!(
            cookie_token.is_none(),
            "Empty refresh_token cookie must parse as None, got {cookie_token:?}"
        );

        // Replicate the handler resolution: cookie (None) falls back to body.
        let body_token = "valid.body.jwt";
        let resolved = cookie_token.as_deref().unwrap_or(body_token);
        assert_eq!(
            resolved, body_token,
            "Valid body token must be used when the cookie is present-but-empty"
        );
    }

    // ==================== Token-resolution helper tests ====================
    //
    // `resolve_refresh_token` and `hash_refresh_token` centralize the
    // cookie-first / body-fallback precedence and the SHA-256 token hashing
    // that `/refresh` and `/logout` (and session lookup) previously hand-rolled
    // as duplicated inline code. The duplication was the root cause of the
    // recent bug cluster (#2190 cookie-blindness, #2205 empty-cookie-shadow),
    // so these tests pin the shared behavior at the resolution level, not just
    // at the `parse_refresh_cookie` level.

    /// A present, non-empty `refresh_token` cookie wins over the body token.
    #[test]
    fn resolve_refresh_token_prefers_cookie() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            "refresh_token=cookie.jwt".parse().unwrap(),
        );
        assert_eq!(resolve_refresh_token(&headers, "body.jwt"), "cookie.jwt");
    }

    /// Regression (#2205): a present-but-EMPTY cookie must NOT shadow the body
    /// token — resolution must fall through to the body value.
    #[test]
    fn resolve_refresh_token_empty_cookie_falls_back_to_body() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            "session_id=abc; refresh_token=; other=y".parse().unwrap(),
        );
        assert_eq!(resolve_refresh_token(&headers, "body.jwt"), "body.jwt");
    }

    /// With no Cookie header at all, resolution uses the body token — the
    /// pre-migration localStorage flow keeps working.
    #[test]
    fn resolve_refresh_token_no_cookie_uses_body() {
        let headers = axum::http::HeaderMap::new();
        assert_eq!(resolve_refresh_token(&headers, "body.jwt"), "body.jwt");
    }

    /// `hash_refresh_token` is a stable SHA-256 hex digest matching the value
    /// stored in `refresh_tokens.token_hash`. Pins a known-answer vector for the
    /// empty input plus determinism/format for a fixed token so an accidental
    /// algorithm swap is caught before it silently breaks session lookups.
    #[test]
    fn hash_refresh_token_is_stable_sha256_hex() {
        // Known-answer vector: SHA-256 of the empty string.
        assert_eq!(
            hash_refresh_token(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let h = hash_refresh_token("some.refresh.jwt");
        assert_eq!(h.len(), 64, "SHA-256 hex must be 64 chars: {h}");
        assert_eq!(
            h,
            hash_refresh_token("some.refresh.jwt"),
            "hashing must be deterministic"
        );
        assert!(
            h.chars().all(|c| c.is_ascii_hexdigit()),
            "digest must be lowercase hex: {h}"
        );
    }
}

// ==================== Email-dispatch rate-limit regression tests ============
//
// Guards the mailbomb / token-clobber fix: `/forgot-password` and
// `/resend-verification` throttle per email. These exercise the exact guard
// helpers the handlers call, so removing the limiter (or bumping the threshold
// out of range) fails the build. Pure in-process — no DB/state needed.
#[cfg(test)]
mod email_dispatch_rate_limit_tests {
    use super::{
        forgot_password_rate_allowed, resend_verification_rate_allowed,
        AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS,
    };

    #[test]
    fn forgot_password_limit_trips_after_threshold() {
        // Unique key so the process-global static is not polluted by other tests.
        let email = "trip-fp-unique@example.test";
        for i in 0..AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS {
            assert!(
                forgot_password_rate_allowed(email),
                "attempt {i} (within the limit) must be allowed"
            );
        }
        assert!(
            !forgot_password_rate_allowed(email),
            "the request past AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS must be blocked (429)"
        );
    }

    #[test]
    fn resend_verification_limit_trips_after_threshold() {
        let email = "trip-rv-unique@example.test";
        for i in 0..AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS {
            assert!(
                resend_verification_rate_allowed(email),
                "attempt {i} (within the limit) must be allowed"
            );
        }
        assert!(
            !resend_verification_rate_allowed(email),
            "the request past AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS must be blocked (429)"
        );
    }

    /// The key is normalised (trim + lowercase) so casing / whitespace can't be
    /// used to sidestep the throttle — the login limiter keys the same way.
    #[test]
    fn email_key_normalisation_prevents_case_bypass() {
        let base = "Case-Bypass-Unique@Example.Test";
        for _ in 0..AUTH_EMAIL_DISPATCH_MAX_ATTEMPTS {
            assert!(forgot_password_rate_allowed(base));
        }
        // A different casing / surrounding whitespace maps to the same key and
        // must remain blocked rather than resetting the counter.
        assert!(
            !forgot_password_rate_allowed("  case-bypass-unique@example.test  "),
            "case/whitespace variant must hit the same throttle bucket"
        );
    }
}

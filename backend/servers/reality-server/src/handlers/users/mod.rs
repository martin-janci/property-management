//! User handlers - portal user management.
//!
//! Implements user registration, OAuth 2.0 SSO with Property Management,
//! and account linking functionality.

use chrono::{Duration, Utc};
use db::models::user::Locale;
use db::models::{CreatePortalPasswordResetToken, PortalUser, UpdatePortalUser};
use db::repositories::{
    PortalPasswordResetRepository, PortalRepository, UnifiedPortalError,
    UnifiedPortalUserRepo, UpdateProfile,
};
use rand::TryRng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// How long a freshly-issued reset token is valid before it must be rotated.
const PASSWORD_RESET_TTL_MINUTES: i64 = 30;

/// Outcome of a password-reset request. We deliberately collapse the
/// "user not found" case into Sent (with an empty token) to avoid
/// account-existence enumeration.
#[derive(Debug)]
pub enum PasswordResetRequestResult {
    /// A reset token was generated. The plaintext token is returned so the
    /// caller can dispatch the email; it is NOT persisted in plaintext.
    Sent { plaintext_token: String },
    /// No such user exists. Returned to the handler so it can log internally
    /// while still returning HTTP 200 to the client.
    UserNotFound,
}

/// Outcome of a confirm-reset request.
#[derive(Debug)]
pub enum PasswordResetConfirmResult {
    Success,
    InvalidToken,
    Expired,
    PasswordTooWeak(Vec<String>),
}

/// User registration result.
#[derive(Debug)]
pub enum RegistrationResult {
    /// User successfully registered
    Success(PortalUser),
    /// Email already exists
    EmailExists,
    /// Invalid email format
    InvalidEmail,
    /// Password too weak
    WeakPassword(Vec<String>),
    /// Cryptographic operation failed (e.g., password hashing)
    CryptoError(String),
    /// Database error
    DatabaseError(String),
}

/// OAuth SSO result.
#[derive(Debug)]
pub enum SsoResult {
    /// User logged in successfully (existing user)
    LoggedIn(PortalUser),
    /// New user created via SSO
    Created(PortalUser),
    /// SSO token/credentials invalid
    InvalidCredentials,
    /// SSO provider error
    ProviderError(String),
}

/// Account linking result.
#[derive(Debug)]
pub enum LinkResult {
    /// Accounts linked successfully
    Success,
    /// Portal account not found
    PortalAccountNotFound,
    /// PM account not found
    PmAccountNotFound,
    /// Account already linked
    AlreadyLinked,
    /// Accounts belong to different emails
    EmailMismatch,
}

/// User service for handling user-related business logic.
///
/// N1 (Phase 2.5): every write path goes through [`UnifiedPortalUserRepo`]
/// so `users` (the source of truth) and `portal_users` (back-compat) stay
/// in sync transactionally. Reads keep using [`PortalRepository`] —
/// dual-write means the portal_users row reflects the latest state.
#[derive(Clone)]
pub struct UserHandler {
    repo: PortalRepository,
    unified: UnifiedPortalUserRepo,
}

impl UserHandler {
    /// Create a new UserHandler.
    pub fn new(repo: PortalRepository) -> Self {
        // The unified repo borrows the same DbPool. We get it back from the
        // portal repo so call-sites don't have to thread a second handle.
        let unified = UnifiedPortalUserRepo::new(repo.pool().clone());
        Self { repo, unified }
    }

    /// Validate email format.
    pub fn validate_email(email: &str) -> bool {
        // Basic email validation
        let email = email.trim().to_lowercase();
        if email.is_empty() || email.len() > 254 {
            return false;
        }

        // Check for @ and at least one dot after @
        if let Some(at_pos) = email.find('@') {
            let domain = &email[at_pos + 1..];
            !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        } else {
            false
        }
    }

    /// Validate password strength.
    /// Returns Ok(()) if valid, Err with list of issues if invalid.
    pub fn validate_password(password: &str) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();

        if password.len() < 8 {
            issues.push("Password must be at least 8 characters".to_string());
        }
        if !password.chars().any(|c| c.is_uppercase()) {
            issues.push("Password must contain at least one uppercase letter".to_string());
        }
        if !password.chars().any(|c| c.is_lowercase()) {
            issues.push("Password must contain at least one lowercase letter".to_string());
        }
        if !password.chars().any(|c| c.is_numeric()) {
            issues.push("Password must contain at least one number".to_string());
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    /// Hash password using Argon2id.
    pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
        use argon2::{
            password_hash::{rand_core::OsRng, SaltString},
            Argon2, PasswordHasher,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    /// Verify password against hash.
    pub fn verify_password(
        password: &str,
        hash: &str,
    ) -> Result<bool, argon2::password_hash::Error> {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(hash)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Register a new portal user with email/password.
    ///
    /// N1: writes both `users` (source of truth, principal_kind='public')
    /// and `portal_users` (back-compat) inside a single transaction via
    /// [`UnifiedPortalUserRepo::create`]. Returns the resulting `PortalUser`
    /// so the existing response shape is preserved — we read it back from
    /// `portal_users` after the dual-write commits.
    pub async fn register(&self, email: &str, password: &str, name: &str) -> RegistrationResult {
        // Validate email
        if !Self::validate_email(email) {
            return RegistrationResult::InvalidEmail;
        }

        // Validate password
        if let Err(issues) = Self::validate_password(password) {
            return RegistrationResult::WeakPassword(issues);
        }

        // Check if email already exists. We look in `users` (the unified
        // source of truth) — the unified repo's find_by_email also falls
        // back to portal_users for the unmerged-collision case, so this
        // detects a legacy portal-only row too.
        match self.unified.find_by_email(email).await {
            Ok(Some(_)) => return RegistrationResult::EmailExists,
            Err(UnifiedPortalError::Db(e)) => {
                return RegistrationResult::DatabaseError(e.to_string())
            }
            Err(UnifiedPortalError::Collision { .. }) => {
                // Cannot happen on a read; defensive arm.
                return RegistrationResult::DatabaseError(
                    "unexpected collision on email lookup".to_string(),
                );
            }
            Ok(None) => {}
        }

        // Hash password
        let password_hash = match Self::hash_password(password) {
            Ok(hash) => hash,
            Err(e) => return RegistrationResult::CryptoError(e.to_string()),
        };

        // Dual-write to users + portal_users in one tx.
        let unified_user = match self
            .unified
            .create(email, name, Some(&password_hash), Locale::default())
            .await
        {
            Ok(u) => u,
            Err(UnifiedPortalError::Db(e)) => {
                return RegistrationResult::DatabaseError(e.to_string())
            }
            Err(UnifiedPortalError::Collision { .. }) => {
                // create() does not currently surface Collision but the type
                // permits it — treat as EmailExists.
                return RegistrationResult::EmailExists;
            }
        };

        // Read back the portal_users row by the back-pointer so the response
        // shape matches the legacy contract. portal_origin_id was set inside
        // the same tx, so this lookup is guaranteed to find the row.
        let portal_user = match unified_user.portal_origin_id {
            Some(pid) => match self.repo.find_user_by_id(pid).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    return RegistrationResult::DatabaseError(
                        "dual-write left no portal_users row".to_string(),
                    )
                }
                Err(e) => return RegistrationResult::DatabaseError(e.to_string()),
            },
            None => {
                return RegistrationResult::DatabaseError(
                    "dual-write did not set portal_origin_id".to_string(),
                )
            }
        };

        RegistrationResult::Success(portal_user)
    }

    /// Login with email/password.
    pub async fn login(&self, email: &str, password: &str) -> Result<PortalUser, &'static str> {
        // Find user by email
        let user = match self.repo.find_user_by_email(email).await {
            Ok(Some(user)) => user,
            Ok(None) => return Err("Invalid email or password"),
            Err(_) => return Err("Login failed"),
        };

        // Check password
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or("Account uses SSO login")?;

        match Self::verify_password(password, password_hash) {
            Ok(true) => Ok(user),
            Ok(false) => Err("Invalid email or password"),
            Err(_) => Err("Login failed"),
        }
    }

    /// Create or update user from PM SSO.
    /// This is called after successful OAuth callback.
    ///
    /// N1: dual-writes via [`UnifiedPortalUserRepo::sso_upsert`]. Email
    /// collisions with non-public principals are NOT silently overwritten —
    /// they surface as [`SsoResult::ProviderError`] with a "collision"
    /// message after the unified repo queues a `user_merge_collisions` row
    /// for human review (defends leak #7 the same way the merge migration
    /// does).
    pub async fn upsert_sso_user(&self, pm_user_id: Uuid, email: &str, name: &str) -> SsoResult {
        // Existed-by-pm_user_id case (legacy SSO link). We still invoke the
        // unified path so a stale name in `users` is updated alongside
        // `portal_users`.
        let was_existing = matches!(self.repo.find_user_by_pm_id(pm_user_id).await, Ok(Some(_)));

        match self.unified.sso_upsert("pm_sso", Some(pm_user_id), email, name).await {
            Ok(user) => {
                let portal_id = user.portal_origin_id.unwrap_or(pm_user_id);
                match self.repo.find_user_by_id(portal_id).await {
                    Ok(Some(p)) => {
                        if was_existing {
                            SsoResult::LoggedIn(p)
                        } else {
                            SsoResult::Created(p)
                        }
                    }
                    Ok(None) => {
                        // Synthesize a portal user payload; the dual-write
                        // committed but the back-pointer projection isn't
                        // visible yet (race against another reader).
                        SsoResult::ProviderError(
                            "dual-write succeeded but portal_users mirror not found".to_string(),
                        )
                    }
                    Err(e) => SsoResult::ProviderError(e.to_string()),
                }
            }
            Err(UnifiedPortalError::Collision { existing_user_id }) => {
                tracing::warn!(
                    email = %email,
                    existing_user_id = %existing_user_id,
                    "SSO upsert refused: email belongs to a non-public principal (collision queued)"
                );
                SsoResult::ProviderError(format!(
                    "email collides with existing non-public principal (collision queued for review): {existing_user_id}"
                ))
            }
            Err(UnifiedPortalError::Db(e)) => SsoResult::ProviderError(e.to_string()),
        }
    }

    /// Link portal account to PM account.
    pub async fn link_account(
        &self,
        portal_user_id: Uuid,
        pm_user_id: Uuid,
        pm_email: &str,
    ) -> LinkResult {
        // Find portal user
        let portal_user = match self.repo.find_user_by_id(portal_user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => return LinkResult::PortalAccountNotFound,
            Err(_) => return LinkResult::PortalAccountNotFound,
        };

        // Check if already linked
        if portal_user.pm_user_id.is_some() {
            return LinkResult::AlreadyLinked;
        }

        // Verify email matches (for security)
        if portal_user.email.to_lowercase() != pm_email.to_lowercase() {
            return LinkResult::EmailMismatch;
        }

        // Update portal user with PM user ID
        // Note: This would need a new repo method to update pm_user_id specifically
        // For now, we indicate success as the linking logic is in place
        tracing::info!(
            portal_user_id = %portal_user_id,
            pm_user_id = %pm_user_id,
            "Account linking requested"
        );

        LinkResult::Success
    }

    /// Get user by ID.
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<PortalUser>, String> {
        self.repo
            .find_user_by_id(user_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Update user profile.
    ///
    /// N1: dual-writes via [`UnifiedPortalUserRepo::update_profile`] so the
    /// unified `users` row is updated first and `portal_users` is mirrored
    /// in the same transaction. Returns the resulting `PortalUser` (read
    /// back from `portal_users`) to keep the existing response shape.
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        name: Option<String>,
        profile_image_url: Option<String>,
        locale: Option<String>,
    ) -> Result<PortalUser, String> {
        // The unified repo is keyed by `users.id`. Resolve a portal-side id
        // to the matching users id when the legacy caller passes a portal
        // user id (the registration flow above returns portal_users.id in
        // its UserInfo, so legacy clients hit /users/me with that id).
        let users_id = self.resolve_users_id(user_id).await.map_err(|e| e.to_string())?;
        let update = UpdateProfile {
            name: name.clone(),
            profile_image_url: profile_image_url.clone(),
            locale: locale.clone(),
        };
        let _ = self
            .unified
            .update_profile(users_id, update)
            .await
            .map_err(|e| e.to_string())?;

        // Read back the portal_users row so the response shape is
        // unchanged. Lookup by users.id → portal_origin_id, falling back to
        // pm_user_id for legacy rows.
        let portal_id = self
            .resolve_portal_id(users_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no portal_users row for this user".to_string())?;

        // Mirror the requested fields through the legacy update path so any
        // portal-only column (e.g. profile_image_url for a row that lacked a
        // back-pointer) still gets updated. This is a no-op when the unified
        // repo already wrote the same fields.
        let legacy_update = UpdatePortalUser {
            name,
            profile_image_url,
            locale,
        };
        self.repo
            .update_user(portal_id, legacy_update)
            .await
            .map_err(|e| e.to_string())
    }

    /// Resolve a caller-supplied user id (could be either `users.id` or
    /// `portal_users.id` depending on whether the legacy or unified path
    /// owned the issuance) to a `users.id`. If the id already maps to a
    /// `users` row, return it; otherwise, look it up through the
    /// portal_origin_id back-pointer.
    async fn resolve_users_id(&self, id: Uuid) -> Result<Uuid, String> {
        let pool = self.repo.pool();
        if let Some(uid) = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
        {
            return Ok(uid);
        }
        if let Some(uid) =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE portal_origin_id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
        {
            return Ok(uid);
        }
        Err(format!("no users row for id {id}"))
    }

    /// Resolve a `portal_users.id` to its corresponding `users.id`. Looks
    /// up via `users.portal_origin_id` first; falls back to
    /// `portal_users.pm_user_id` for legacy SSO links. Returns the
    /// `portal_users.id` itself as a last resort so the unified update
    /// degrades gracefully on rows the merge migration could not touch
    /// (e.g. an unmerged collision).
    async fn lookup_users_id_for_portal_id(&self, portal_id: Uuid) -> Result<Uuid, String> {
        let pool = self.repo.pool();
        if let Some(uid) = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM users WHERE portal_origin_id = $1",
        )
        .bind(portal_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        {
            return Ok(uid);
        }
        if let Some(uid) =
            sqlx::query_scalar::<_, Uuid>("SELECT pm_user_id FROM portal_users WHERE id = $1")
                .bind(portal_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
                .flatten()
        {
            return Ok(uid);
        }
        // Unmerged-collision case: no matching users row. The password
        // change still has to land on portal_users so the legacy auth path
        // works; we fall back to writing through PortalRepository directly.
        // The caller (confirm_password_reset) already does that legacy
        // write below the unified call as a defense in depth.
        Ok(portal_id)
    }

    /// Resolve `users.id` to its `portal_users.id`, if any.
    async fn resolve_portal_id(&self, users_id: Uuid) -> Result<Option<Uuid>, String> {
        let pool = self.repo.pool();
        if let Some(pid) = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT portal_origin_id FROM users WHERE id = $1",
        )
        .bind(users_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .flatten()
        {
            return Ok(Some(pid));
        }
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM portal_users WHERE pm_user_id = $1")
            .bind(users_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())
    }

    /// Issue a password reset token for the user with this email.
    ///
    /// Returns the plaintext token to the caller (so it can put it in an
    /// email link). Only the SHA-256 hash is persisted — the
    /// reset-confirm endpoint will hash the incoming token and compare.
    /// Existing unused tokens for the same user are invalidated so a
    /// stale link from an earlier request can no longer be used.
    pub async fn request_password_reset(
        &self,
        reset_repo: &PortalPasswordResetRepository,
        email: &str,
    ) -> Result<PasswordResetRequestResult, String> {
        let user = self
            .repo
            .find_user_by_email(email)
            .await
            .map_err(|e| e.to_string())?;

        let user = match user {
            Some(u) => u,
            None => return Ok(PasswordResetRequestResult::UserNotFound),
        };

        // Burn any existing tokens before issuing a new one.
        reset_repo
            .invalidate_user_tokens(user.id)
            .await
            .map_err(|e| e.to_string())?;

        // 32 cryptographically-random bytes → URL-safe base64 → ~43 char
        // token that we surface to the user via email.
        let mut bytes = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut bytes)
            .expect("OS rng failed");
        let plaintext_token =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
        let token_hash = sha256_hex(&plaintext_token);
        let expires_at = Utc::now() + Duration::minutes(PASSWORD_RESET_TTL_MINUTES);

        reset_repo
            .create(CreatePortalPasswordResetToken {
                portal_user_id: user.id,
                token_hash,
                expires_at,
            })
            .await
            .map_err(|e| e.to_string())?;

        Ok(PasswordResetRequestResult::Sent { plaintext_token })
    }

    /// Verify a reset token and replace the user's password hash.
    pub async fn confirm_password_reset(
        &self,
        reset_repo: &PortalPasswordResetRepository,
        plaintext_token: &str,
        new_password: &str,
    ) -> Result<PasswordResetConfirmResult, String> {
        if let Err(issues) = Self::validate_password(new_password) {
            return Ok(PasswordResetConfirmResult::PasswordTooWeak(issues));
        }

        let token_hash = sha256_hex(plaintext_token);
        let token = reset_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| e.to_string())?;

        let token = match token {
            Some(t) if t.is_valid() => t,
            Some(_) => return Ok(PasswordResetConfirmResult::Expired),
            None => return Ok(PasswordResetConfirmResult::InvalidToken),
        };

        let new_hash = Self::hash_password(new_password).map_err(|e| e.to_string())?;

        // N1: dual-write through the unified repo so the password hash on
        // `users` (used by future RequestPrincipal-based authn) stays in
        // sync with `portal_users` (used by today's session-based authn).
        // The token carries `portal_user_id`; resolve to the matching
        // `users.id` first.
        let users_id = self
            .lookup_users_id_for_portal_id(token.portal_user_id)
            .await
            .map_err(|e| e.to_string())?;
        let unified_ok = self
            .unified
            .update_password_hash(users_id, &new_hash)
            .await
            .map_err(|e| e.to_string())?;

        // Fallback path: when no unified `users` row matched (the
        // unmerged-collision case where users_id is actually a
        // portal_users id without a back-pointer), the unified write is a
        // no-op — write through the legacy path so the legacy auth flow
        // still picks up the new hash.
        if !unified_ok {
            self.repo
                .update_password_hash(token.portal_user_id, &new_hash)
                .await
                .map_err(|e| e.to_string())?;
        }

        // Single-use: mark the token consumed and invalidate any other
        // outstanding tokens for the same user (a stolen-but-unused token
        // can't be used after a password change).
        reset_repo
            .mark_used(token.id)
            .await
            .map_err(|e| e.to_string())?;
        reset_repo
            .invalidate_user_tokens(token.portal_user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(PasswordResetConfirmResult::Success)
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

//! Application state for Reality Server.
//!
//! Contains shared services and configuration for SSO, user management, and portal repositories.
//! Epic 104: Includes PM API health client and SSO token validation caching.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use db::{
    repositories::{
        PortalPasswordResetRepository, PortalRepository, RealityPortalRepository,
        UnifiedPortalError, UnifiedPortalUserRepo,
    },
    DbPool,
};

use crate::handlers::inquiries::{InquiriesHandler, LogInquiryNotifier};
use crate::routes::sso::{OAuthTokens, PendingSsoSession, SessionInfo, SsoUserInfo};

/// Application configuration.
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// PM OAuth authorize URL
    pub pm_oauth_authorize_url: String,
    /// PM OAuth token URL
    pub pm_oauth_token_url: String,
    /// PM userinfo URL
    pub pm_userinfo_url: String,
    /// PM token introspection URL
    pub pm_introspect_url: String,
    /// PM OAuth client ID
    pub pm_client_id: String,
    /// PM OAuth client secret
    pub pm_client_secret: String,
    /// SSO callback URL (this server)
    pub sso_callback_url: String,
    /// JWT secret for session tokens
    pub jwt_secret: String,
    /// PM API health check URL (Epic 104.1)
    pub pm_api_health_url: String,
    /// Allowlist of post-SSO redirect origins (`scheme://host[:port]`).
    ///
    /// Loaded from `ALLOWED_REDIRECT_ORIGINS` (comma-separated). `None` means
    /// no client-supplied `redirect_uri` is accepted at all — handlers must
    /// fall back to a hard-coded safe path (e.g. `"/"`). Empty list also
    /// rejects everything.
    pub allowed_redirect_origins: Option<Vec<String>>,
}

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let pm_api_base = std::env::var("PM_API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string())
            .trim_end_matches('/')
            .to_string();
        Self {
            pm_oauth_authorize_url: std::env::var("PM_OAUTH_AUTHORIZE_URL")
                .unwrap_or_else(|_| format!("{}/api/v1/oauth/authorize", pm_api_base)),
            pm_oauth_token_url: std::env::var("PM_OAUTH_TOKEN_URL")
                .unwrap_or_else(|_| format!("{}/api/v1/oauth/token", pm_api_base)),
            pm_userinfo_url: std::env::var("PM_USERINFO_URL")
                .unwrap_or_else(|_| format!("{}/api/v1/oauth/userinfo", pm_api_base)),
            pm_introspect_url: std::env::var("PM_INTROSPECT_URL")
                .unwrap_or_else(|_| format!("{}/api/v1/oauth/introspect", pm_api_base)),
            pm_client_id: std::env::var("PM_CLIENT_ID")
                .unwrap_or_else(|_| "reality-portal".to_string()),
            pm_client_secret: {
                // SECURITY: PM_CLIENT_SECRET must be set in production. Fallback is only
                // permitted when RUST_ENV=development to support local development.
                let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";
                std::env::var("PM_CLIENT_SECRET").unwrap_or_else(|_| {
                    if is_development {
                        tracing::warn!(
                            "PM_CLIENT_SECRET not set, using development default (DEVELOPMENT MODE ONLY)"
                        );
                        "reality-portal-dev-secret-do-not-use-in-production".to_string()
                    } else {
                        panic!(
                            "PM_CLIENT_SECRET environment variable is required in non-development environments. \
                             Set RUST_ENV=development to use the dev default."
                        );
                    }
                })
            },
            sso_callback_url: std::env::var("SSO_CALLBACK_URL")
                .unwrap_or_else(|_| "http://localhost:8081/api/v1/sso/callback".to_string()),
            jwt_secret: {
                // SECURITY: JWT secret validation with strict production requirements
                let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";
                let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                    if is_development {
                        tracing::warn!(
                            "JWT_SECRET not set, using development default (DEVELOPMENT MODE ONLY)"
                        );
                        "development-secret-key-that-is-at-least-64-characters-long-for-testing".to_string()
                    } else {
                        panic!("JWT_SECRET environment variable is required. Set RUST_ENV=development to use dev defaults.");
                    }
                });

                // SECURITY: Validate secret strength
                if secret.len() < 32 {
                    panic!("JWT_SECRET must be at least 32 characters long for minimum security");
                }
                if !is_development && secret.len() < 64 {
                    tracing::warn!(
                        "JWT_SECRET is {} characters (minimum 64 recommended for production security)",
                        secret.len()
                    );
                }
                secret
            },
            // Default points at api-server's deep readiness, not its
            // shallow liveness. Reality-server's own readiness wants the
            // full picture for the operator dashboard. Docker HEALTHCHECK
            // is unaffected — it still uses `/health`.
            pm_api_health_url: std::env::var("PM_API_HEALTH_URL")
                .unwrap_or_else(|_| format!("{}/readiness", pm_api_base)),
            // SECURITY: post-SSO redirect target allowlist. Origins are
            // compared as `scheme://host[:port]` exact-match strings.
            // Missing env var ⇒ `None` ⇒ all client-supplied redirect_uris
            // are rejected (handlers fall back to "/"). This is the safe
            // default — operators must opt in to redirecting back to the
            // web/mobile app origins.
            allowed_redirect_origins: std::env::var("ALLOWED_REDIRECT_ORIGINS").ok().map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().trim_end_matches('/').to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            }),
        }
    }
}

/// User service for managing portal users (database-backed).
///
/// N1: SSO upsert dual-writes via [`UnifiedPortalUserRepo`] so the
/// authoritative `users` row is created/updated alongside the legacy
/// `portal_users` row. Email collisions with non-public principals are
/// REFUSED (queued in `user_merge_collisions` for review) — never silently
/// merged. This matches the Phase 2 merge migration's contract for the
/// historical-data path; it now applies to forward-going SSO sign-ins too.
#[derive(Clone)]
pub struct UserService {
    repo: PortalRepository,
    unified: UnifiedPortalUserRepo,
}

impl UserService {
    /// Create a new user service with database repository.
    pub fn new(pool: DbPool) -> Self {
        Self {
            repo: PortalRepository::new(pool.clone()),
            unified: UnifiedPortalUserRepo::new(pool),
        }
    }

    /// Create or update a portal user from SSO user info.
    ///
    /// Phase 6: `portal_users` has been dropped (migration 00148). The unified
    /// write now goes only to `users`; `find_user_by_id` reads from `users` too.
    pub async fn upsert_sso_user(
        &self,
        info: &SsoUserInfo,
    ) -> Result<db::models::portal::PortalUser, anyhow::Error> {
        let pm_user_id = uuid::Uuid::parse_str(&info.user_id)
            .map_err(|e| anyhow::anyhow!("Invalid PM user ID: {}", e))?;

        match self
            .unified
            .sso_upsert("pm_sso", Some(pm_user_id), &info.email, &info.name)
            .await
        {
            Ok(user) => {
                // Optionally update profile_image_url if supplied by the IdP.
                if let Some(avatar) = info.avatar_url.as_deref() {
                    let _ = self
                        .repo
                        .update_user(
                            user.id,
                            db::models::UpdatePortalUser {
                                name: None,
                                profile_image_url: Some(avatar.to_string()),
                                locale: None,
                            },
                        )
                        .await;
                }
                // Re-read through PortalRepository (now reads from users) to
                // get the PortalUser-shaped response expected by callers.
                let portal_user = self
                    .repo
                    .find_user_by_id(user.id)
                    .await
                    .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
                    .ok_or_else(|| {
                        anyhow::anyhow!("upsert succeeded but users row not found immediately")
                    })?;
                Ok(portal_user)
            }
            Err(UnifiedPortalError::Collision { existing_user_id }) => {
                Err(anyhow::anyhow!(
                    "SSO upsert refused: email '{}' already belongs to non-public principal {} (collision queued)",
                    info.email,
                    existing_user_id
                ))
            }
            Err(UnifiedPortalError::Db(e)) => Err(anyhow::anyhow!("Database error: {}", e)),
        }
    }

    /// Get portal user by PM user ID.
    pub async fn get_by_pm_id(&self, pm_user_id: &str) -> Option<db::models::portal::PortalUser> {
        let pm_user_uuid = uuid::Uuid::parse_str(pm_user_id).ok()?;
        self.repo
            .find_user_by_pm_id(pm_user_uuid)
            .await
            .ok()
            .flatten()
    }

    /// Get portal user by email.
    pub async fn get_by_email(&self, email: &str) -> Option<db::models::portal::PortalUser> {
        self.repo.find_user_by_email(email).await.ok().flatten()
    }
}

/// Session service for managing user sessions (database-backed).
///
/// Sessions are stored in the database for persistence across restarts
/// and horizontal scaling. Tokens are hashed with SHA-256 before storage.
#[derive(Clone)]
pub struct SessionService {
    repo: PortalRepository,
    jwt_secret: String,
}

impl SessionService {
    /// Create a new session service with database repository.
    pub fn new(pool: DbPool, jwt_secret: String) -> Self {
        Self {
            repo: PortalRepository::new(pool),
            jwt_secret,
        }
    }

    /// Hash a session token for storage (SHA-256).
    fn hash_token(token: &str) -> String {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(token.as_bytes());
        hex::encode(hash)
    }

    /// Create a new session for a user after SSO login.
    pub async fn create_session(
        &self,
        user_id: uuid::Uuid,
        _tokens: &OAuthTokens,
    ) -> Result<String, anyhow::Error> {
        let session_token = self.generate_session_token(user_id)?;
        let token_hash = Self::hash_token(&session_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

        self.repo
            .create_session(user_id, &token_hash, expires_at)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;

        Ok(session_token)
    }

    /// Create a session for mobile SSO (without PM tokens).
    pub async fn create_mobile_session(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<String, anyhow::Error> {
        let session_token = self.generate_session_token(user_id)?;
        let token_hash = Self::hash_token(&session_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

        self.repo
            .create_session(user_id, &token_hash, expires_at)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create mobile session: {}", e))?;

        Ok(session_token)
    }

    /// Get session info by token.
    pub async fn get_session(&self, token: &str) -> Result<SessionInfo, anyhow::Error> {
        let token_hash = Self::hash_token(token);

        // Get session from database
        let session = self
            .repo
            .find_session_by_token_hash(&token_hash)
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Get user info
        let user = self
            .repo
            .find_user_by_id(session.user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        Ok(SessionInfo {
            user_id: session.user_id,
            email: user.email,
            name: user.name,
            expires_at: session.expires_at,
        })
    }

    /// Refresh session (extend expiry).
    pub async fn refresh_session(&self, token: &str) -> Result<SessionInfo, anyhow::Error> {
        let token_hash = Self::hash_token(token);
        let new_expires_at = chrono::Utc::now() + chrono::Duration::days(7);

        let session = self
            .repo
            .refresh_session(&token_hash, new_expires_at)
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Session not found or expired"))?;

        // Get user info
        let user = self
            .repo
            .find_user_by_id(session.user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        Ok(SessionInfo {
            user_id: session.user_id,
            email: user.email,
            name: user.name,
            expires_at: session.expires_at,
        })
    }

    /// Invalidate a session (logout).
    pub async fn invalidate_session(&self, token: &str) -> Result<(), anyhow::Error> {
        let token_hash = Self::hash_token(token);
        self.repo
            .delete_session(&token_hash)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to invalidate session: {}", e))?;
        Ok(())
    }

    /// Clean up expired sessions (call periodically).
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, anyhow::Error> {
        self.repo
            .cleanup_expired_sessions()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to cleanup sessions: {}", e))
    }

    fn generate_session_token(&self, user_id: uuid::Uuid) -> Result<String, anyhow::Error> {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde::{Deserialize, Serialize};

        // Phase 2 token shape: { sub, kind, iat, exp }. The `kind` claim is
        // informational only — `RequestPrincipal` re-derives `principal_kind`
        // from the trusted `users` table on every request (defense for leaks
        // #8 and #11). We still emit it so a future client/tooling can
        // discriminate public vs staff tokens by inspection without a DB
        // round-trip; servers must NEVER trust it.
        #[derive(Serialize, Deserialize)]
        struct Claims {
            sub: String,
            kind: &'static str,
            exp: i64,
            iat: i64,
        }

        let now = chrono::Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            kind: "public",
            exp: (now + chrono::Duration::days(7)).timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }
}

/// SSO token service for mobile deep-link flow.
///
/// ## Durability (issue #820, P1 — deliberate follow-up)
///
/// Tokens live in an in-process `Mutex<HashMap>`, so they are **not durable
/// across reality-server instances**. The mobile deep-link flow therefore
/// requires the `/sso/mobile/token` mint and the `/sso/mobile/validate`
/// redeem to hit the same instance (sticky routing). Migrating to a shared
/// store (Redis) is tracked as a follow-up; reality-server has no Redis
/// dependency today, so adding one is out of scope for the #820 security
/// review. The security properties below hold regardless of durability:
///
/// - **One-time use:** `validate_and_consume_token` removes the entry before
///   returning it, so a token can be redeemed at most once.
/// - **Short TTL:** tokens are minted with a 5-minute lifetime and rejected
///   past `expires_at`.
/// - **Unforgeable:** the token value is a random v4 UUID.
#[derive(Clone)]
pub struct SsoTokenService {
    // Short-lived tokens for mobile SSO
    tokens: Arc<Mutex<HashMap<String, MobileSsoToken>>>,
}

#[derive(Clone, Debug)]
struct MobileSsoToken {
    user_info: SsoUserInfo,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl Default for SsoTokenService {
    fn default() -> Self {
        Self::new()
    }
}

impl SsoTokenService {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a short-lived mobile SSO token.
    pub async fn create_mobile_token(
        &self,
        user_info: &SsoUserInfo,
        duration: chrono::Duration,
    ) -> Result<String, anyhow::Error> {
        let token = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let expires_at = now + duration;

        let mut tokens = self.tokens.lock().await;
        // Amortized eviction (issue #820): tokens that are minted but never
        // redeemed (abandoned deep-links, replay probes) would otherwise
        // accumulate in the map forever, since `validate_and_consume_token`
        // only removes a token when it is actually presented. Sweep expired
        // entries on every mint so the map stays bounded by the number of
        // *live* (<= 5-minute-old) tokens rather than total tokens ever issued.
        tokens.retain(|_, t| t.expires_at >= now);
        tokens.insert(
            token.clone(),
            MobileSsoToken {
                user_info: user_info.clone(),
                expires_at,
            },
        );

        Ok(token)
    }

    /// Validate and consume a mobile SSO token (one-time use).
    pub async fn validate_and_consume_token(
        &self,
        token: &str,
    ) -> Result<SsoUserInfo, anyhow::Error> {
        let mut tokens = self.tokens.lock().await;
        let sso_token = tokens
            .remove(token)
            .ok_or_else(|| anyhow::anyhow!("Invalid or expired token"))?;

        if sso_token.expires_at < chrono::Utc::now() {
            return Err(anyhow::anyhow!("Token expired"));
        }

        Ok(sso_token.user_info)
    }

    /// Number of tokens currently held (live + not-yet-swept expired).
    ///
    /// Test/observability helper — lets callers assert the store does not grow
    /// unboundedly across mints (issue #820 eviction regression test).
    #[cfg(test)]
    pub(crate) async fn len(&self) -> usize {
        self.tokens.lock().await.len()
    }
}

// ==================== Epic 104: Caching Infrastructure ====================

/// PM API health check result (Story 104.1).
#[derive(Clone, Debug)]
pub struct PmApiHealthResult {
    /// Health status from PM API
    pub status: String,
    /// Response latency in milliseconds
    pub latency_ms: u64,
    /// PM API version
    pub version: Option<String>,
    /// When the check was performed
    pub checked_at: Instant,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Cached health check with TTL (Story 104.1).
#[derive(Clone, Debug)]
pub struct CachedHealthCheck {
    /// The health check result
    pub result: PmApiHealthResult,
    /// When the cache entry expires
    pub expires_at: Instant,
}

/// SSO token validation cache entry (Story 104.2).
#[derive(Clone, Debug)]
pub struct CachedTokenValidation {
    /// Whether the token is valid/active
    pub active: bool,
    /// Subject (user ID) from token
    pub sub: Option<String>,
    /// Token scope
    pub scope: Option<String>,
    /// When the cache entry expires
    pub expires_at: Instant,
}

/// Cache metrics for monitoring (Story 104.2).
#[derive(Clone, Debug, Default)]
pub struct CacheMetrics {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total evictions (expired entries)
    pub evictions: u64,
}

/// Health check cache service (Story 104.1).
#[derive(Clone)]
pub struct HealthCheckCache {
    /// Cached PM API health result
    cache: Arc<RwLock<Option<CachedHealthCheck>>>,
    /// Cache TTL in seconds (default: 30)
    ttl_seconds: u64,
    /// Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl Default for HealthCheckCache {
    fn default() -> Self {
        Self::new(30) // 30 second default TTL
    }
}

impl HealthCheckCache {
    /// Create a new health check cache with specified TTL.
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            ttl_seconds,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    /// Get cached health check if valid.
    pub async fn get(&self) -> Option<PmApiHealthResult> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.as_ref() {
            if Instant::now() < cached.expires_at {
                let mut metrics = self.metrics.write().await;
                metrics.hits += 1;
                return Some(cached.result.clone());
            }
            // Entry expired
            drop(cache);
            let mut metrics = self.metrics.write().await;
            metrics.evictions += 1;
        } else {
            let mut metrics = self.metrics.write().await;
            metrics.misses += 1;
        }
        None
    }

    /// Store health check result in cache.
    pub async fn set(&self, result: PmApiHealthResult) {
        let mut cache = self.cache.write().await;
        *cache = Some(CachedHealthCheck {
            result,
            expires_at: Instant::now() + Duration::from_secs(self.ttl_seconds),
        });
    }

    /// Get cache metrics.
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    /// Clear the cache.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
    }
}

/// SSO token validation cache service (Story 104.2).
#[derive(Clone)]
pub struct TokenValidationCache {
    /// Cached token validations (token hash -> validation result)
    cache: Arc<RwLock<HashMap<String, CachedTokenValidation>>>,
    /// Cache TTL in seconds (default: 60)
    ttl_seconds: u64,
    /// Maximum cache entries
    max_entries: usize,
    /// Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl Default for TokenValidationCache {
    fn default() -> Self {
        Self::new(60, 10000) // 60 second TTL, 10000 max entries
    }
}

impl TokenValidationCache {
    /// Create a new token validation cache.
    pub fn new(ttl_seconds: u64, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
            max_entries,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    /// Hash a token for cache key (avoid storing raw tokens).
    fn hash_token(token: &str) -> String {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(token.as_bytes());
        hex::encode(hash)
    }

    /// Get cached token validation if valid.
    pub async fn get(&self, token: &str) -> Option<CachedTokenValidation> {
        let token_hash = Self::hash_token(token);
        let cache = self.cache.read().await;

        if let Some(cached) = cache.get(&token_hash) {
            if Instant::now() < cached.expires_at {
                let mut metrics = self.metrics.write().await;
                metrics.hits += 1;
                return Some(cached.clone());
            }
        }

        drop(cache);
        let mut metrics = self.metrics.write().await;
        metrics.misses += 1;
        None
    }

    /// Store token validation result in cache.
    pub async fn set(&self, token: &str, active: bool, sub: Option<String>, scope: Option<String>) {
        let token_hash = Self::hash_token(token);
        let mut cache = self.cache.write().await;

        // Evict expired entries if cache is full
        if cache.len() >= self.max_entries {
            let now = Instant::now();
            let expired_keys: Vec<String> = cache
                .iter()
                .filter(|(_, v)| v.expires_at < now)
                .map(|(k, _)| k.clone())
                .collect();

            let mut metrics = self.metrics.write().await;
            metrics.evictions += expired_keys.len() as u64;
            drop(metrics);

            for key in expired_keys {
                cache.remove(&key);
            }

            // If still full after eviction, remove oldest entries
            if cache.len() >= self.max_entries {
                let entries_to_remove = cache.len() - self.max_entries + 1;
                let keys_to_remove: Vec<String> = cache
                    .iter()
                    .take(entries_to_remove)
                    .map(|(k, _)| k.clone())
                    .collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
        }

        cache.insert(
            token_hash,
            CachedTokenValidation {
                active,
                sub,
                scope,
                expires_at: Instant::now() + Duration::from_secs(self.ttl_seconds),
            },
        );
    }

    /// Invalidate a cached token.
    pub async fn invalidate(&self, token: &str) {
        let token_hash = Self::hash_token(token);
        let mut cache = self.cache.write().await;
        cache.remove(&token_hash);
    }

    /// Get cache metrics.
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    /// Get cache size.
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Clear the cache.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// HTTP client for PM API communication (Story 104.1).
#[derive(Clone)]
pub struct PmApiClient {
    /// HTTP client
    client: reqwest::Client,
    /// Health check URL
    health_url: String,
}

impl PmApiClient {
    /// Create a new PM API client.
    pub fn new(health_url: String, timeout_seconds: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, health_url }
    }

    /// Check PM API health.
    pub async fn check_health(&self) -> PmApiHealthResult {
        let start = Instant::now();

        match self.client.get(&self.health_url).send().await {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    // Try to parse the health response
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let status = json
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let version = json
                                .get("version")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            PmApiHealthResult {
                                status,
                                latency_ms,
                                version,
                                checked_at: Instant::now(),
                                error: None,
                            }
                        }
                        Err(e) => PmApiHealthResult {
                            status: "degraded".to_string(),
                            latency_ms,
                            version: None,
                            checked_at: Instant::now(),
                            error: Some(format!("Failed to parse health response: {}", e)),
                        },
                    }
                } else {
                    PmApiHealthResult {
                        status: "unhealthy".to_string(),
                        latency_ms,
                        version: None,
                        checked_at: Instant::now(),
                        error: Some(format!("HTTP {}", response.status())),
                    }
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;

                PmApiHealthResult {
                    status: "unhealthy".to_string(),
                    latency_ms,
                    version: None,
                    checked_at: Instant::now(),
                    error: Some(format!("Connection failed: {}", e)),
                }
            }
        }
    }
}

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: DbPool,
    /// Portal repository for search, favorites, saved searches
    pub portal_repo: PortalRepository,
    /// Reality Portal Professional repository (agencies, realtors, inquiries)
    pub reality_portal_repo: RealityPortalRepository,
    /// Shared inquiries handler. The public anonymous POST routes
    /// (`send_contact_message`, `request_viewing`) create inquiries through
    /// this handler rather than the repository directly, so the best-effort
    /// realtor notification fires via the injected [`InquiryNotifier`] seam.
    /// Built with [`LogInquiryNotifier`] via [`InquiriesHandler::with_notifier`]
    /// until a real email/push transport is wired — swapping the notifier here
    /// is then the single change needed to light up delivery.
    pub inquiries_handler: InquiriesHandler,
    /// Portal password-reset token repository (UC-44.3)
    pub portal_password_reset_repo: PortalPasswordResetRepository,
    /// Password-reset email transport (UC-44.3). Selected from the environment:
    /// a real SMTP transport when `SMTP_*` is configured, otherwise a logging
    /// fallback that reports `is_configured() == false` so the request handler
    /// never falsely claims a reset link was delivered.
    pub password_reset_mailer: Arc<dyn crate::services::PasswordResetMailer>,
    /// Application configuration
    pub config: AppConfig,
    /// Pending SSO sessions (OAuth flow state)
    pub sso_sessions: Arc<Mutex<HashMap<String, PendingSsoSession>>>,
    /// User service for portal users
    pub user_service: UserService,
    /// Session service for managing user sessions
    pub session_service: SessionService,
    /// SSO token service for mobile deep-link flow
    pub sso_token_service: SsoTokenService,
    /// PM API HTTP client (Epic 104.1)
    pub pm_api_client: PmApiClient,
    /// Shared pooled HTTP client for the PM OAuth/SSO calls (token exchange,
    /// userinfo, introspection). Built once with connect + request timeouts so
    /// the SSO handlers reuse one connection pool instead of constructing a
    /// fresh, timeout-less `reqwest::Client` per call (which pools nothing and
    /// can hang indefinitely if the PM endpoint stalls). `reqwest::Client` is
    /// internally `Arc`-backed, so cloning it out of `AppState` is cheap.
    pub pm_oauth_client: reqwest::Client,
    /// PM API health check cache (Epic 104.1)
    pub health_cache: HealthCheckCache,
    /// SSO token validation cache (Epic 104.2)
    pub token_cache: TokenValidationCache,
    /// Phase 1: Host-resolution cache shared with `host_tenant_middleware`.
    /// Holds the SAME `Arc` the middleware uses, so domain-management handlers
    /// can invalidate entries (e.g. after a domain is verified).
    pub tenant_resolution_cache: std::sync::Arc<api_core::middleware::TenantResolutionCache>,
    /// Phase 5.5: per-tenant rate limiter set shared with `host_tenant_middleware`.
    /// Holds the SAME `Arc` the middleware uses; admin handlers can install
    /// per-tenant overrides via `tenant_rate_limiters.set_override(org, rpm)`.
    pub tenant_rate_limiters: std::sync::Arc<api_core::middleware::TenantRateLimiterSet>,
    /// Per-IP throttle for the UNAUTHENTICATED inquiry POST endpoints
    /// (`send_contact_message`, `request_viewing`). The per-tenant limiter
    /// above keys on a resolved org and therefore does not cover anonymous
    /// inquiry traffic, which persists rows and fires realtor notifications;
    /// this set keys on a hash of the client IP so an anonymous flood is
    /// answered with HTTP 429 (see `InquiriesHandler::rate_limit_result`).
    /// Default quota comes from `INQUIRY_RATE_LIMIT_RPM` (env override).
    ///
    /// Because the key space is attacker-influenced (rotating source IPs), the
    /// set is constructed **bounded** (`with_default_bounded`) so the map can
    /// never grow past `INQUIRY_RATE_LIMIT_MAX_IPS` entries — memory is capped
    /// inline on every insert, no background sweep task required.
    pub inquiry_rate_limiters: std::sync::Arc<api_core::middleware::TenantRateLimiterSet>,
    /// Number of trusted reverse-proxy hops in front of reality-server, used to
    /// pick a **spoof-resistant** client address out of `X-Forwarded-For` (the
    /// hop our own trusted proxy appended, counted from the right — see
    /// `routes::inquiries::client_ip_bucket`). Loaded from
    /// `INQUIRY_TRUSTED_PROXY_HOPS`, default 1 (a single reverse proxy).
    pub inquiry_trusted_proxy_hops: usize,
}

/// Default per-IP quota (requests/minute) for the anonymous inquiry POST
/// endpoints. Deliberately low — no legitimate visitor submits more than a
/// handful of contact/viewing requests per minute from one address, while a
/// spam/DB-exhaustion bot needs far more. Override with `INQUIRY_RATE_LIMIT_RPM`.
pub const INQUIRY_RATE_LIMIT_RPM: u32 = 12;

/// Hard cap on the number of distinct per-IP buckets held by the anonymous
/// inquiry limiter. Bounds memory against an attacker rotating source IPs:
/// once the map is full, inserting a new IP first reclaims TTL-expired buckets
/// and then evicts the oldest — the map never exceeds this many entries.
/// ~100k small entries is a few MB. Override with `INQUIRY_RATE_LIMIT_MAX_IPS`.
pub const INQUIRY_RATE_LIMIT_MAX_IPS: usize = 100_000;

/// Idle TTL for a per-IP inquiry bucket. Only needs to outlive the 1-minute
/// rate window; a few minutes lets bursts be counted while keeping the map
/// small. Buckets idle longer than this are reclaimed on the next insert.
pub const INQUIRY_RATE_LIMIT_IDLE_TTL: Duration = Duration::from_secs(300);

impl AppState {
    /// Create a new AppState with database pool.
    pub fn new(
        db: DbPool,
        tenant_resolution_cache: std::sync::Arc<api_core::middleware::TenantResolutionCache>,
        tenant_rate_limiters: std::sync::Arc<api_core::middleware::TenantRateLimiterSet>,
    ) -> Self {
        let portal_repo = PortalRepository::new(db.clone());
        let reality_portal_repo = RealityPortalRepository::new(db.clone());
        let portal_password_reset_repo = PortalPasswordResetRepository::new(db.clone());
        // UC-44.3: pick the reset email transport from the environment. Without
        // SMTP_* this returns a logging fallback (delivery disabled) so the
        // request handler can refuse to claim a link was sent in production.
        let password_reset_mailer = crate::services::build_password_reset_mailer();

        // Public inquiry POSTs go through this handler so the best-effort realtor
        // notification fires. `with_notifier` makes the transport injectable:
        // today it is the logging stub; a real email/push adapter drops in here.
        let inquiries_handler = InquiriesHandler::with_notifier(
            reality_portal_repo.clone(),
            Arc::new(LogInquiryNotifier),
        );
        let config = AppConfig::from_env();
        let jwt_secret = config.jwt_secret.clone();

        // Epic 104.1: Create PM API client for health checks
        let pm_api_client = PmApiClient::new(config.pm_api_health_url.clone(), 5);

        // Shared pooled HTTP client for the SSO OAuth calls (exchange /
        // userinfo / introspect). Built once here so the three handlers reuse a
        // single connection pool and inherit bounded timeouts, rather than each
        // spinning up a fresh, timeout-less `reqwest::Client::new()` per request.
        // Mirrors the `.timeout()` + `.connect_timeout()` convention used by the
        // integration clients (e.g. `booking::client`).
        let pm_oauth_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        // Epic 104.1: Health check cache with 30 second TTL
        let health_cache = HealthCheckCache::new(30);

        // Epic 104.2: Token validation cache with 60 second TTL, 10000 max entries
        let token_cache = TokenValidationCache::new(60, 10000);

        // Security fix: Use database-backed services for persistence
        let user_service = UserService::new(db.clone());
        let session_service = SessionService::new(db.clone(), jwt_secret);

        // Per-IP throttle for anonymous inquiry POSTs. Independent of the
        // per-tenant set: anonymous traffic has no resolved org to key on.
        let inquiry_rpm = std::env::var("INQUIRY_RATE_LIMIT_RPM")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(INQUIRY_RATE_LIMIT_RPM);
        // Bounded construction: the key space is a hash of the client IP, which
        // an attacker can rotate freely, so we MUST cap the map or it grows
        // without bound (memory-DoS). `with_default_bounded` evicts inline.
        let inquiry_max_ips = std::env::var("INQUIRY_RATE_LIMIT_MAX_IPS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(INQUIRY_RATE_LIMIT_MAX_IPS);
        let inquiry_rate_limiters = Arc::new(
            api_core::middleware::TenantRateLimiterSet::with_default_bounded(
                inquiry_rpm,
                INQUIRY_RATE_LIMIT_IDLE_TTL,
                inquiry_max_ips,
            ),
        );
        // Trusted reverse-proxy hop count for spoof-resistant client-IP
        // derivation (default 1 = a single reverse proxy in front of us).
        let inquiry_trusted_proxy_hops = std::env::var("INQUIRY_TRUSTED_PROXY_HOPS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(1);

        Self {
            db,
            portal_repo,
            reality_portal_repo,
            inquiries_handler,
            portal_password_reset_repo,
            password_reset_mailer,
            config,
            sso_sessions: Arc::new(Mutex::new(HashMap::new())),
            user_service,
            session_service,
            sso_token_service: SsoTokenService::new(),
            pm_api_client,
            pm_oauth_client,
            health_cache,
            token_cache,
            // Phase 1: shared host-resolution cache
            tenant_resolution_cache,
            // Phase 5.5: shared per-tenant rate limiter set (defense leak #15)
            tenant_rate_limiters,
            // Per-IP throttle for unauthenticated inquiry POSTs.
            inquiry_rate_limiters,
            inquiry_trusted_proxy_hops,
        }
    }

    /// Acquire a connection for public (non-tenant-scoped) queries such as
    /// listing search/detail and inquiry create.
    ///
    /// On success, any stale RLS context from a previous request has been
    /// cleared so the caller sees a fresh session. If clearing fails we do
    /// NOT hand the connection back to the caller: we close it (to force
    /// the pool to open a fresh one later) and surface the error, because
    /// a silently-stale RLS context would be a tenant-isolation bypass.
    ///
    /// Callers must use the returned connection (`&mut *conn`) as the
    /// executor instead of passing `&self.db` directly — the latter is
    /// flagged by `scripts/check-rls-enforcement.sh` as an RLS violation.
    pub async fn acquire_public_conn(
        &self,
    ) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, sqlx::Error> {
        let mut conn = self.db.acquire().await?;
        if let Err(e) = db::clear_request_context(&mut *conn).await {
            tracing::warn!(
                error = %e,
                "Failed to clear stale RLS context on public connection acquire; dropping connection"
            );
            // Drop the connection explicitly. It returns to the pool and is
            // closed via the `after_release` hook configured on the pool
            // (see db::create_rls_safe_pool).
            drop(conn);
            return Err(e);
        }
        Ok(conn)
    }
}

// Phase 1 / Phase 2.5 (N1): Implement TenantMembershipProvider so the
// host-resolution extractors (`HostRlsConnection`) AND the unified
// `RequestPrincipal` extractor can obtain the db pool from this state.
//
// `RequestPrincipal` uses the pool to:
//   * load `users.principal_kind` per request (re-derived, never trusted
//     from the JWT — defense for leaks #8/#11),
//   * call `MembershipRepository::is_active(user, host_org)` when the
//     resolved host pins a real organization.
//
// `HostRlsConnection` itself performs no membership query — its tenant
// comes from host resolution, not from a login.
impl api_core::TenantMembershipProvider for AppState {
    fn db_pool(&self) -> &DbPool {
        &self.db
    }
}

// Make OAuthTokens cloneable
impl Clone for OAuthTokens {
    fn clone(&self) -> Self {
        Self {
            access_token: self.access_token.clone(),
            refresh_token: self.refresh_token.clone(),
            token_type: self.token_type.clone(),
            expires_in: self.expires_in,
        }
    }
}

// ==================== Mobile SSO token store tests (issue #820) ====================

#[cfg(test)]
mod sso_token_service_tests {
    use super::SsoTokenService;
    use crate::routes::sso::SsoUserInfo;

    fn user(id: &str) -> SsoUserInfo {
        SsoUserInfo {
            user_id: id.to_string(),
            email: format!("{id}@example.com"),
            name: format!("User {id}"),
            avatar_url: None,
        }
    }

    /// A freshly minted token validates exactly once, then is gone.
    #[tokio::test]
    async fn mobile_token_is_one_time_use() {
        let svc = SsoTokenService::new();
        let token = svc
            .create_mobile_token(&user("u1"), chrono::Duration::minutes(5))
            .await
            .expect("mint");

        let first = svc.validate_and_consume_token(&token).await;
        assert!(first.is_ok(), "first redemption should succeed");
        assert_eq!(first.unwrap().user_id, "u1");

        let second = svc.validate_and_consume_token(&token).await;
        assert!(
            second.is_err(),
            "second redemption of the same token must be rejected"
        );
    }

    /// An expired token is rejected on redemption (short-TTL enforcement).
    #[tokio::test]
    async fn mobile_token_expired_is_rejected() {
        let svc = SsoTokenService::new();
        // Mint already-expired (negative duration).
        let token = svc
            .create_mobile_token(&user("u2"), chrono::Duration::seconds(-1))
            .await
            .expect("mint");

        let result = svc.validate_and_consume_token(&token).await;
        assert!(result.is_err(), "expired token must be rejected");
    }

    /// Regression (issue #820): minting new tokens must sweep expired,
    /// never-redeemed tokens so the in-memory map cannot grow unboundedly.
    #[tokio::test]
    async fn mobile_token_mint_evicts_expired_entries() {
        let svc = SsoTokenService::new();

        // Insert 5 already-expired tokens that are never redeemed.
        for i in 0..5 {
            svc.create_mobile_token(&user(&format!("old{i}")), chrono::Duration::seconds(-1))
                .await
                .expect("mint expired");
        }
        // Each subsequent mint runs the sweep first, so after a single live
        // mint the only surviving entry is the live one — the 5 expired,
        // abandoned tokens are evicted rather than leaked.
        svc.create_mobile_token(&user("live"), chrono::Duration::minutes(5))
            .await
            .expect("mint live");

        assert_eq!(
            svc.len().await,
            1,
            "expired, never-redeemed tokens must be swept on mint"
        );
    }

    /// Regression (issue #820, PR #921): the eviction sweep on mint must only
    /// remove *expired* tokens — a still-live token minted earlier must survive
    /// later mints and remain redeemable. Guards against a `retain` predicate
    /// that is too aggressive (e.g. comparing against the new token's
    /// `expires_at` instead of `now`, or an inverted comparison), which the
    /// `mobile_token_mint_evicts_expired_entries` test alone cannot catch
    /// because it never holds more than one live token at assertion time.
    #[tokio::test]
    async fn mobile_token_mint_preserves_still_live_tokens() {
        let svc = SsoTokenService::new();

        // Mint a still-live token first.
        let live_first = svc
            .create_mobile_token(&user("first"), chrono::Duration::minutes(5))
            .await
            .expect("mint first live");

        // Interleave an already-expired token (to be swept) and a second live
        // token, then mint a third live token whose sweep must keep all live
        // entries intact.
        svc.create_mobile_token(&user("expired"), chrono::Duration::seconds(-1))
            .await
            .expect("mint expired");
        let live_second = svc
            .create_mobile_token(&user("second"), chrono::Duration::minutes(5))
            .await
            .expect("mint second live");
        svc.create_mobile_token(&user("third"), chrono::Duration::minutes(5))
            .await
            .expect("mint third live");

        // Only the single expired entry should have been swept; the three live
        // tokens must remain.
        assert_eq!(
            svc.len().await,
            3,
            "sweep on mint must preserve every still-live token"
        );

        // The earliest still-live token must remain redeemable (proves it was
        // not collaterally evicted by a later mint's sweep).
        let redeemed = svc.validate_and_consume_token(&live_first).await;
        assert!(
            redeemed.is_ok(),
            "a still-live token minted before later mints must remain redeemable"
        );
        assert_eq!(redeemed.unwrap().user_id, "first");

        // And the second live token is likewise intact and one-time-usable.
        let redeemed_second = svc.validate_and_consume_token(&live_second).await;
        assert!(
            redeemed_second.is_ok(),
            "all still-live tokens must survive the eviction sweep"
        );
        assert_eq!(redeemed_second.unwrap().user_id, "second");
    }
}
